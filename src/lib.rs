//! Arma 3 plugin wrapper around `minerva-server`: pushes Arma state into the
//! server's [`minerva_server::StateCache`] and executes commands dispatched
//! back to it. See `AGENTS.md` for the SQF side.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use arma_rs::{Context, ContextState, Extension, Group, arma};
use minerva_server::{
    CommandId, CommandOutcome, CommandSink, GroupId, ServerConfig, ServerHandle, UnitId, proto,
};

mod convert;
mod sink;

use sink::ArmaCommandSink;

/// Cloning just clones the inner `Arc`s, so a clone obtained from a
/// `Context` borrow can outlive that borrow (needed in [`cmd_start`], which
/// must move its `Context` into the [`ArmaCommandSink`] it stores).
#[derive(Default, Clone)]
struct AppState {
    server: Arc<Mutex<Option<ServerHandle>>>,
}

fn lock(mutex: &Mutex<Option<ServerHandle>>) -> MutexGuard<'_, Option<ServerHandle>> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn app_state(ctx: &Context) -> Result<AppState, String> {
    ctx.global()
        .get::<AppState>()
        .cloned()
        .ok_or_else(|| "minerva: extension state not initialized".to_string())
}

/// Runs `f` with the running server's handle, or fails if `start` hasn't
/// been called (or `stop` has).
fn with_server<T>(
    ctx: &Context,
    f: impl FnOnce(&ServerHandle) -> Result<T, String>,
) -> Result<T, String> {
    let app = app_state(ctx)?;
    let guard = lock(&app.server);
    match guard.as_ref() {
        Some(handle) => f(handle),
        None => Err("minerva: not started".to_string()),
    }
}

#[arma]
pub fn init() -> Extension {
    Extension::build()
        .version(env!("CARGO_PKG_VERSION").to_string())
        .state(AppState::default())
        .command("start", cmd_start)
        .command("stop", cmd_stop)
        .command("reset", cmd_reset)
        .group(
            "sim",
            Group::new()
                .command("info", cmd_sim_info)
                .command("state", cmd_sim_state),
        )
        .group(
            "group",
            Group::new()
                .command("upsert", cmd_group_upsert)
                .command("remove", cmd_group_remove),
        )
        .group("unit", Group::new().command("remove", cmd_unit_remove))
        .group("command", Group::new().command("ack", cmd_command_ack))
        .finish()
}

/// Idempotent: if already started, returns the existing bound address
/// without touching `addr`.
fn cmd_start(ctx: Context, addr: String) -> Result<String, String> {
    // Owned (Arc-backed) clone so it outlives the `&ctx` borrow it came
    // from — `ctx` itself is moved into the sink below. `guard` then
    // borrows from this owned `app`, not from `ctx`, so it can stay held
    // across that move: the whole idempotency-check + spawn + store stays
    // one critical section, so two racing `start` calls can't both spawn a
    // server.
    let app = app_state(&ctx)?;
    let mut guard = lock(&app.server);
    if let Some(handle) = guard.as_ref() {
        return Ok(handle.local_addr().to_string());
    }

    let socket_addr: SocketAddr = addr
        .parse()
        .map_err(|err| format!("invalid address {addr:?}: {err}"))?;
    let sink: Arc<dyn CommandSink> = Arc::new(ArmaCommandSink::new(ctx));
    let config = ServerConfig {
        addr: socket_addr,
        ..ServerConfig::default()
    };
    let handle = ServerHandle::spawn(config, sink).map_err(|err| err.to_string())?;
    let bound = handle.local_addr().to_string();
    *guard = Some(handle);
    Ok(bound)
}

fn cmd_stop(ctx: Context) -> Result<bool, String> {
    let app = app_state(&ctx)?;
    Ok(lock(&app.server).take().is_some())
}

fn cmd_reset(ctx: Context) -> Result<bool, String> {
    let app = app_state(&ctx)?;
    let guard = lock(&app.server);
    match guard.as_ref() {
        Some(handle) => {
            handle.state().clear();
            handle.dispatcher().cancel_all();
            Ok(true)
        }
        None => Ok(false),
    }
}

fn cmd_sim_info(
    ctx: Context,
    world_name: String,
    world_size: [f64; 2],
    factions: Vec<(String, String)>,
    start_date: [i64; 5],
) -> Result<(), String> {
    let factions = factions
        .into_iter()
        .map(|(name, side)| {
            Ok(proto::Faction {
                name,
                side: convert::side_from_str(&side)? as i32,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    with_server(&ctx, |handle| {
        handle.state().set_simulation_info(proto::SimulationInfo {
            world_name,
            world_size: Some(proto::WorldSize {
                x_meters: world_size[0],
                y_meters: world_size[1],
            }),
            factions,
            simulation_start_sim_time: convert::unix_time_from_arma_date(start_date),
            simulation_start_real_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or_default(),
        });
        Ok(())
    })
}

fn cmd_sim_state(
    ctx: Context,
    date: [i64; 5],
    time_acceleration: f32,
    overcast: f32,
    wind: [f32; 3],
) -> Result<(), String> {
    let (wind_speed, wind_direction) = convert::wind_speed_direction(wind);
    with_server(&ctx, |handle| {
        handle
            .state()
            .set_simulation_state(proto::SimulationStateUpdate {
                simulation_time: convert::unix_time_from_arma_date(date),
                time_acceleration,
                weather: Some(proto::Weather {
                    overcast,
                    wind_speed,
                    wind_direction,
                }),
            });
        Ok(())
    })
}

/// A `group:upsert` unit arg: `(id, kind, type, posASL, dir, velocity,
/// damage)`. No `group_id` -- the server fills that in from the containing
/// group's own id.
type GroupUpsertUnit = (String, String, String, [f64; 3], f32, [f32; 3], f32);

#[allow(clippy::too_many_arguments)]
fn cmd_group_upsert(
    ctx: Context,
    id: String,
    side: String,
    readiness: [f32; 3],
    has_task: bool,
    waypoints: Vec<(String, [f64; 3])>,
    units: Vec<GroupUpsertUnit>,
) -> Result<(), String> {
    let side = convert::side_from_str(&side)?;
    let waypoints = waypoints
        .into_iter()
        .map(|(waypoint_type, position)| proto::Waypoint {
            r#type: convert::waypoint_type_from_str(&waypoint_type) as i32,
            position: Some(convert::position_from_asl(position)),
        })
        .collect();
    let units = units
        .into_iter()
        .map(
            |(unit_id, kind, unit_type, pos_asl, dir, velocity, damage)| {
                convert::unit_from_parts(unit_id, &kind, unit_type, pos_asl, dir, velocity, damage)
            },
        )
        .collect();
    with_server(&ctx, |handle| {
        handle.state().upsert_group_with_units(
            proto::Group {
                id,
                side: side as i32,
                readiness: Some(proto::GroupReadiness {
                    fuel_state: readiness[0],
                    ammo_state: readiness[1],
                    health_state: readiness[2],
                }),
                has_task,
                waypoints,
                units: Vec::new(),
            },
            units,
        );
        Ok(())
    })
}

fn cmd_group_remove(ctx: Context, id: String) -> Result<bool, String> {
    with_server(&ctx, |handle| {
        Ok(handle.state().remove_group(&GroupId::from(id)).is_some())
    })
}

fn cmd_unit_remove(ctx: Context, id: String) -> Result<bool, String> {
    with_server(&ctx, |handle| {
        Ok(handle.state().remove_unit(&UnitId::from(id)).is_some())
    })
}

fn cmd_command_ack(
    ctx: Context,
    command_id: u64,
    ok: bool,
    reason: String,
) -> Result<bool, String> {
    with_server(&ctx, |handle| {
        let outcome = if ok {
            CommandOutcome::Success
        } else {
            CommandOutcome::Failure(reason)
        };
        Ok(handle
            .dispatcher()
            .complete(CommandId::from(command_id), outcome))
    })
}
