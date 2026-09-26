use arma_rs::{Context, Value};
use minerva_server::proto::CommandTarget;
use minerva_server::{Command, CommandId, CommandSink};

/// Dispatches commands into Arma via an `ExtensionCallback`
/// (<https://community.bistudio.com/wiki/Arma_3:_Mission_Event_Handlers#ExtensionCallback>),
/// picked up on the SQF side by `fnc_onCallback`/`fnc_executeCommand`. The
/// callback data is `[id, type, groupId, ...]`, `id` as a decimal string (to
/// round-trip exactly through `command:ack` regardless of magnitude), `type`
/// one of "move"/"search_and_destroy"/"defend_zone"/"patrol"/"support".
pub struct ArmaCommandSink {
    context: Context,
}

impl ArmaCommandSink {
    pub fn new(context: Context) -> Self {
        Self { context }
    }
}

impl CommandSink for ArmaCommandSink {
    fn dispatch(&self, id: CommandId, command: &Command) {
        let id: u64 = id.into();
        let (kind, group_id, extra) = match command {
            Command::Move { group_id, position } => {
                ("move", group_id, command_target_values(position))
            }
            Command::SearchAndDestroy { group_id, position } => (
                "search_and_destroy",
                group_id,
                command_target_values(position),
            ),
            Command::DefendZone {
                group_id,
                zone_id,
                position,
            } => {
                let mut extra = vec![Value::String(zone_id.clone())];
                if let Some(position) = position {
                    extra.extend(command_target_values(position));
                }
                ("defend_zone", group_id, extra)
            }
            Command::Patrol {
                group_id,
                waypoints,
                loop_,
            } => {
                let mut extra = vec![Value::Boolean(*loop_)];
                extra.extend(waypoints.iter().flat_map(command_target_values));
                ("patrol", group_id, extra)
            }
            Command::Support {
                supporter_group_id,
                supported_group_id,
                support_type,
            } => (
                "support",
                supporter_group_id,
                vec![
                    Value::String(supported_group_id.to_string()),
                    Value::String(support_type.clone()),
                ],
            ),
        };

        let mut payload = vec![
            Value::String(id.to_string()),
            Value::String(kind.to_string()),
            Value::String(group_id.to_string()),
        ];
        payload.extend(extra);

        if let Err(err) =
            self.context
                .callback_data("minerva", "command", Value::direct(Value::Array(payload)))
        {
            tracing::error!(%err, "minerva: failed to send command callback");
        }
    }
}

/// `z` is absent for a target the caller only knows in the map plane (e.g.
/// a click on a 2D map) -- encoded as an empty array, vs. a one-element
/// array when present, since callback payloads have no null. See
/// `fnc_executeCommand.sqf` for how each side is placed.
fn command_target_values(target: &CommandTarget) -> Vec<Value> {
    let z = match target.z {
        Some(z) => vec![Value::Number(z)],
        None => Vec::new(),
    };
    vec![
        Value::Number(target.x),
        Value::Number(target.y),
        Value::Array(z),
    ]
}
