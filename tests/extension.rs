//! Exercises the extension the way Arma does: through `arma_rs`'s testing
//! harness (string-marshalled calls), plus a real tonic client against the
//! server `start` actually spins up.

use std::fmt::Debug;
use std::time::Duration;

use arma_rs::Value;
use minerva::init;
use minerva_server::proto::command_service_client::CommandServiceClient;
use minerva_server::proto::group_service_client::GroupServiceClient;
use minerva_server::proto::send_command_request::Command as CommandOneof;
use minerva_server::proto::simulation_service_client::SimulationServiceClient;
use minerva_server::proto::unit_service_client::UnitServiceClient;
use minerva_server::proto::{
    CommandResult, CommandTarget, GetSimulationInfoRequest, GetUnitRequest, ListGroupsRequest,
    ListUnitsRequest, MoveCommand, SendCommandRequest, SubscribeGroupUpdatesRequest,
    SubscribeSimulationUpdatesRequest,
};
use tokio_stream::StreamExt;

fn expect<T, E: Debug>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => panic!("unexpected error: {err:?}"),
    }
}

fn expect_some<T>(value: Option<T>) -> T {
    match value {
        Some(value) => value,
        None => panic!("expected Some, got None"),
    }
}

/// Wraps `s` the way a real `callExtension` call would: string arguments
/// arrive quoted (`FromArma::from_arma` for `String` strips one layer of
/// quoting), everything else (numbers, arrays, bools) is fine unquoted.
fn quoted(s: &str) -> String {
    format!("\"{s}\"")
}

/// A `group:upsert` unit tuple literal, the shape `fnc_pushGroup.sqf` builds
/// per tracked unit: `[id, kind, type, posASL, dir, velocity, damage]`. No
/// `group_id` -- the server fills that in from the containing group.
#[allow(clippy::too_many_arguments)]
fn unit_literal(
    id: &str,
    kind: &str,
    unit_type: &str,
    pos_asl: [f64; 3],
    dir: f32,
    velocity: [f32; 3],
    damage: f32,
) -> String {
    format!(
        "[{},{},{},[{},{},{}],{dir},[{},{},{}],{damage}]",
        quoted(id),
        quoted(kind),
        quoted(unit_type),
        pos_asl[0],
        pos_asl[1],
        pos_asl[2],
        velocity[0],
        velocity[1],
        velocity[2],
    )
}

fn units_literal(units: &[String]) -> String {
    format!("[{}]", units.join(","))
}

/// A full `group:upsert` call's args: id, side, readiness (full/idle),
/// no task, no waypoints, and `units` (pass `"[]"` for none).
fn group_upsert_args(id: &str, side: &str, units: &str) -> Vec<String> {
    vec![
        quoted(id),
        quoted(side),
        "[1,1,1]".to_string(),
        "false".to_string(),
        "[]".to_string(),
        units.to_string(),
    ]
}

#[test]
fn start_is_idempotent() {
    let extension = init().testing();
    let (addr1, code1) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(code1, 0);
    let (addr2, code2) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(code2, 0);
    assert_eq!(
        addr1, addr2,
        "start must return the same address once already running"
    );
}

#[test]
fn start_rejects_invalid_address() {
    let extension = init().testing();
    let (result, code) = extension.call("start", Some(vec![quoted("not-an-address")]));
    assert_ne!(code, 0);
    assert!(result.contains("invalid address"), "{result}");
}

#[test]
fn stop_reports_whether_running() {
    let extension = init().testing();
    let (result, code) = extension.call("stop", None);
    assert_eq!(code, 0);
    assert_eq!(result, "false");

    let (_, start_code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(start_code, 0);

    let (result, code) = extension.call("stop", None);
    assert_eq!(code, 0);
    assert_eq!(result, "true");
}

#[test]
fn reset_reports_whether_running() {
    let extension = init().testing();
    let (result, code) = extension.call("reset", None);
    assert_eq!(code, 0);
    assert_eq!(result, "false");

    let (_, start_code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(start_code, 0);

    let (result, code) = extension.call("reset", None);
    assert_eq!(code, 0);
    assert_eq!(result, "true");
}

#[test]
fn commands_fail_before_start() {
    let extension = init().testing();
    let (result, code) =
        extension.call("group:upsert", Some(group_upsert_args("g1", "WEST", "[]")));
    assert_ne!(code, 0);
    assert!(result.contains("not started"), "{result}");
}

#[test]
fn group_upsert_rejects_wrong_arg_count() {
    let extension = init().testing();
    let (_, start_code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(start_code, 0);
    let (_, code) = extension.call("group:upsert", Some(vec![quoted("g1")]));
    assert_ne!(code, 0);
}

#[test]
fn group_upsert_rejects_unknown_side() {
    let extension = init().testing();
    let (_, start_code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(start_code, 0);
    let (result, code) =
        extension.call("group:upsert", Some(group_upsert_args("g1", "LOGIC", "[]")));
    assert_ne!(code, 0);
    assert!(result.contains("unknown side"), "{result}");
}

#[test]
fn group_upsert_then_remove_happy_path() {
    let extension = init().testing();
    let (_, start_code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(start_code, 0);
    let (_, upsert_code) =
        extension.call("group:upsert", Some(group_upsert_args("g1", "WEST", "[]")));
    assert_eq!(upsert_code, 0);

    let (removed, remove_code) = extension.call("group:remove", Some(vec![quoted("g1")]));
    assert_eq!(remove_code, 0);
    assert_eq!(removed, "true");

    let (removed_again, remove_code) = extension.call("group:remove", Some(vec![quoted("g1")]));
    assert_eq!(remove_code, 0);
    assert_eq!(removed_again, "false");
}

#[test]
fn command_ack_unknown_id_returns_false() {
    let extension = init().testing();
    let (_, start_code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(start_code, 0);
    let (result, code) = extension.call(
        "command:ack",
        Some(vec![quoted("9999"), "true".to_string(), quoted("")]),
    );
    assert_eq!(code, 0);
    assert_eq!(result, "false");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn group_upsert_with_units_is_visible_via_grpc() {
    let extension = init().testing();
    let (bound, code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(code, 0);

    let units = units_literal(&[
        unit_literal(
            "u1",
            "INFANTRY",
            "B_Soldier_F",
            [100.0, 200.0, 300.0],
            45.0,
            [1.0, 2.0, 3.0],
            0.25,
        ),
        unit_literal(
            "u2",
            "INFANTRY",
            "B_Soldier_F",
            [101.0, 201.0, 301.0],
            90.0,
            [0.0, 0.0, 0.0],
            0.0,
        ),
    ]);
    let (_, code) = extension.call(
        "group:upsert",
        Some(group_upsert_args("g1", "WEST", &units)),
    );
    assert_eq!(code, 0);

    // Visible embedded in the group...
    let mut groups = expect(GroupServiceClient::connect(format!("http://{bound}")).await);
    let listed = expect(groups.list_groups(ListGroupsRequest { side: None }).await).into_inner();
    assert_eq!(listed.groups.len(), 1);
    assert_eq!(
        listed.groups[0]
            .units
            .iter()
            .map(|u| u.id.as_str())
            .collect::<Vec<_>>(),
        vec!["u1", "u2"]
    );

    // ...and, since the server fills group_id from the containing group
    // regardless of what a unit arrived with, via UnitService too.
    let mut unit_client = expect(UnitServiceClient::connect(format!("http://{bound}")).await);
    let response = expect(
        unit_client
            .get_unit(GetUnitRequest {
                id: "u1".to_string(),
            })
            .await,
    )
    .into_inner();
    let unit = expect_some(response.unit);
    assert_eq!(unit.group_id, "g1");
    let position = expect_some(unit.state.and_then(|s| s.position));
    assert_eq!((position.x, position.y, position.z), (100.0, 200.0, 300.0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn group_upsert_repeated_unchanged_call_emits_nothing() {
    let extension = init().testing();
    let (bound, code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(code, 0);

    let units = units_literal(&[unit_literal(
        "u1",
        "INFANTRY",
        "B_Soldier_F",
        [0.0, 0.0, 0.0],
        0.0,
        [0.0, 0.0, 0.0],
        0.0,
    )]);
    let (_, code) = extension.call(
        "group:upsert",
        Some(group_upsert_args("g1", "WEST", &units)),
    );
    assert_eq!(code, 0);

    let mut groups = expect(GroupServiceClient::connect(format!("http://{bound}")).await);
    let mut stream = expect(
        groups
            .subscribe_group_updates(SubscribeGroupUpdatesRequest { side: None })
            .await,
    )
    .into_inner();
    // Drain the snapshot from the upsert above.
    expect(expect_some(stream.next().await));

    // The exact same call again (same group fields, same unit state): its
    // joined value is unchanged, so this must not re-broadcast. A second,
    // genuinely different upsert afterward proves the stream is still
    // live and would have delivered the repeat if it were going to.
    let (_, code) = extension.call(
        "group:upsert",
        Some(group_upsert_args("g1", "WEST", &units)),
    );
    assert_eq!(code, 0);
    let (_, code) = extension.call("group:remove", Some(vec![quoted("g1")]));
    assert_eq!(code, 0);

    let next = expect(expect_some(stream.next().await));
    use minerva_server::proto::subscribe_group_updates_response::Event;
    assert!(
        matches!(next.event, Some(Event::RemovedId(_))),
        "expected the repeat to be skipped and the removal to arrive next, got {next:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn group_upsert_moves_a_unit_between_groups() {
    let extension = init().testing();
    let (bound, code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(code, 0);

    let unit = unit_literal(
        "u1",
        "INFANTRY",
        "B_Soldier_F",
        [0.0, 0.0, 0.0],
        0.0,
        [0.0, 0.0, 0.0],
        0.0,
    );
    let (_, code) = extension.call(
        "group:upsert",
        Some(group_upsert_args(
            "g1",
            "WEST",
            &units_literal(std::slice::from_ref(&unit)),
        )),
    );
    assert_eq!(code, 0);
    // g1 pushes an empty unit list (as it would once u1 boards a vehicle
    // owned by a different group) while g2 picks u1 up in the same tick.
    let (_, code) = extension.call("group:upsert", Some(group_upsert_args("g1", "WEST", "[]")));
    assert_eq!(code, 0);
    let (_, code) = extension.call(
        "group:upsert",
        Some(group_upsert_args("g2", "WEST", &units_literal(&[unit]))),
    );
    assert_eq!(code, 0);

    let mut unit_client = expect(UnitServiceClient::connect(format!("http://{bound}")).await);
    let g1_units = expect(
        unit_client
            .list_units(ListUnitsRequest {
                side: None,
                group_id: Some("g1".to_string()),
            })
            .await,
    )
    .into_inner();
    assert_eq!(g1_units.units.len(), 0);

    let g2_units = expect(
        unit_client
            .list_units(ListUnitsRequest {
                side: None,
                group_id: Some("g2".to_string()),
            })
            .await,
    )
    .into_inner();
    assert_eq!(
        g2_units
            .units
            .iter()
            .map(|u| u.id.as_str())
            .collect::<Vec<_>>(),
        vec!["u1"]
    );
}

/// Regression test for a real bug: `wind` (passed by `fnc_pushState`) is a
/// 3-element `[x, y, z]` vector, not 2 -- `cmd_sim_state`'s param type used
/// to require exactly 2, so every real `sim:state` call failed arg parsing
/// and simulation state was never updated.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sim_info_and_state_are_visible_via_grpc() {
    let extension = init().testing();
    let (bound, code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(code, 0);

    let (_, code) = extension.call(
        "sim:info",
        Some(vec![
            quoted("Altis"),
            "[15360,10240]".to_string(),
            "[]".to_string(),
            "[2026,1,1,0,0]".to_string(),
        ]),
    );
    assert_eq!(code, 0);

    // wind = [3, 4, 0] -> speed = hypot(3, 4) = 5.
    let (_, code) = extension.call(
        "sim:state",
        Some(vec![
            "[2026,1,1,0,5]".to_string(),
            "1".to_string(),
            "0.5".to_string(),
            "[3,4,0]".to_string(),
        ]),
    );
    assert_eq!(code, 0);

    let mut simulation = expect(SimulationServiceClient::connect(format!("http://{bound}")).await);

    let info = expect(
        simulation
            .get_simulation_info(GetSimulationInfoRequest {})
            .await,
    )
    .into_inner();
    let info = expect_some(info.info);
    assert_eq!(info.world_name, "Altis");
    let world_size = expect_some(info.world_size);
    assert_eq!(
        (world_size.x_meters, world_size.y_meters),
        (15360.0, 10240.0)
    );

    let mut stream = expect(
        simulation
            .subscribe_simulation_updates(SubscribeSimulationUpdatesRequest {})
            .await,
    )
    .into_inner();
    let snapshot = expect(expect_some(stream.next().await));
    let weather = expect_some(expect_some(snapshot.state).weather);
    assert!((weather.wind_speed - 5.0).abs() < 1e-3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn send_command_acked_via_callback_and_ack() {
    let extension = init().testing();
    let (bound, code) = extension.call("start", Some(vec![quoted("127.0.0.1:0")]));
    assert_eq!(code, 0);
    let (_, code) = extension.call("group:upsert", Some(group_upsert_args("g1", "WEST", "[]")));
    assert_eq!(code, 0);

    let mut commands = expect(CommandServiceClient::connect(format!("http://{bound}")).await);
    let request = SendCommandRequest {
        command: Some(CommandOneof::Move(MoveCommand {
            position: Some(CommandTarget {
                x: 1.0,
                y: 2.0,
                z: Some(3.0),
            }),
            group_id: "g1".to_string(),
        })),
    };
    let call = tokio::spawn(async move { commands.send_command(request).await });

    // Blocks this task/thread (not the spawned one above) until the sink's
    // callback arrives; safe since `extension` never needs to cross a
    // thread/task boundary itself (crossbeam channels don't care).
    let callback = extension.callback_handler::<_, Option<Value>, String>(
        |name, func, data| {
            if name == "minerva" && func == "command" {
                arma_rs::testing::Result::Ok(data)
            } else {
                arma_rs::testing::Result::Continue
            }
        },
        Duration::from_secs(5),
    );
    let data = match callback {
        arma_rs::testing::Result::Ok(data) => data,
        other => panic!("unexpected callback result: {other:?}"),
    };
    let items = match data {
        Some(Value::Array(items)) => items,
        other => panic!("unexpected callback payload: {other:?}"),
    };
    let id = match items.first() {
        Some(Value::String(s)) => s.clone(),
        other => panic!("expected string id as the first callback element, got {other:?}"),
    };
    let kind = match items.get(1) {
        Some(Value::String(s)) => s.clone(),
        other => panic!("expected string kind as the second callback element, got {other:?}"),
    };
    assert_eq!(kind, "move");

    let (_, ack_code) = extension.call(
        "command:ack",
        Some(vec![quoted(&id), "true".to_string(), quoted("")]),
    );
    assert_eq!(ack_code, 0);

    let response = expect(expect(call.await)).into_inner();
    assert_eq!(response.result, CommandResult::Success as i32);
}
