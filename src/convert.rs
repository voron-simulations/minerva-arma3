//! Arma value -> protocol/domain type conversions. Kept out of `lib.rs` so
//! the command handlers stay focused on wiring, not data munging.

use minerva_server::proto;

pub fn side_from_str(side: &str) -> Result<proto::Side, String> {
    match side {
        "WEST" => Ok(proto::Side::Blufor),
        "EAST" => Ok(proto::Side::Opfor),
        "GUER" => Ok(proto::Side::Independent),
        "CIV" => Ok(proto::Side::Civilian),
        other => Err(format!("unknown side: {other}")),
    }
}

pub fn unit_category_from_str(kind: &str) -> proto::UnitCategory {
    match kind {
        "INFANTRY" => proto::UnitCategory::Infantry,
        "VEHICLE" => proto::UnitCategory::Vehicle,
        "PLANE" => proto::UnitCategory::Plane,
        "HELICOPTER" => proto::UnitCategory::Helicopter,
        _ => proto::UnitCategory::Unspecified,
    }
}

pub fn waypoint_type_from_str(waypoint_type: &str) -> proto::WaypointType {
    match waypoint_type {
        "MOVE" => proto::WaypointType::Move,
        "SAD" => proto::WaypointType::Attack,
        "GUARD" => proto::WaypointType::Guard,
        "LOITER" => proto::WaypointType::Loiter,
        _ => proto::WaypointType::Unspecified,
    }
}

/// `getDamage`/`damage` report 0 (undamaged) to 1 (destroyed); the protocol
/// reports health the other way around.
pub fn health_from_damage(damage: f32) -> f32 {
    (1.0 - damage).clamp(0.0, 1.0)
}

pub fn position_from_asl(pos: [f64; 3]) -> proto::Position {
    proto::Position {
        x: pos[0],
        y: pos[1],
        z: pos[2],
    }
}

/// `wind` reports a `[x, y]` vector in m/s (x: west-east, y: south-north).
/// The protocol wants speed and direction (radians, counterclockwise from
/// east) separately.
pub fn wind_speed_direction(wind: [f32; 2]) -> (f32, f32) {
    let [x, y] = wind;
    (x.hypot(y), y.atan2(x))
}

/// Converts an Arma `date` array (`[year, month, day, hour, minute]`) to a
/// unix timestamp (seconds, truncated to the minute — `date` has no
/// second-level precision).
pub fn unix_time_from_arma_date(date: [i64; 5]) -> i64 {
    let [year, month, day, hour, minute] = date;
    days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60
}

/// Howard Hinnant's `days_from_civil`: days since the unix epoch for a
/// proleptic-Gregorian calendar date. <https://howardhinnant.github.io/date_algorithms.html>
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // [0, 399]
    let mp = (m + 9) % 12; // [0, 11]
    let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn side_from_str_maps_known_sides() {
        assert_eq!(side_from_str("WEST"), Ok(proto::Side::Blufor));
        assert_eq!(side_from_str("EAST"), Ok(proto::Side::Opfor));
        assert_eq!(side_from_str("GUER"), Ok(proto::Side::Independent));
        assert_eq!(side_from_str("CIV"), Ok(proto::Side::Civilian));
        assert!(side_from_str("LOGIC").is_err());
    }

    #[test]
    fn unit_category_from_str_defaults_to_unspecified() {
        assert_eq!(
            unit_category_from_str("INFANTRY"),
            proto::UnitCategory::Infantry
        );
        assert_eq!(
            unit_category_from_str("SUBMARINE"),
            proto::UnitCategory::Unspecified
        );
    }

    #[test]
    fn health_from_damage_inverts_and_clamps() {
        assert_eq!(health_from_damage(0.0), 1.0);
        assert_eq!(health_from_damage(1.0), 0.0);
        assert_eq!(health_from_damage(0.25), 0.75);
        assert_eq!(health_from_damage(1.5), 0.0);
        assert_eq!(health_from_damage(-0.5), 1.0);
    }

    #[test]
    fn wind_speed_direction_matches_common_headings() {
        let (speed, direction) = wind_speed_direction([1.0, 0.0]);
        assert!((speed - 1.0).abs() < 1e-6);
        assert!(direction.abs() < 1e-6); // due east -> 0 radians

        let (speed, direction) = wind_speed_direction([0.0, 1.0]);
        assert!((speed - 1.0).abs() < 1e-6);
        assert!((direction - std::f32::consts::FRAC_PI_2).abs() < 1e-6); // due north -> pi/2
    }

    #[test]
    fn unix_time_from_arma_date_matches_known_instant() {
        // 2026-01-01 00:00 UTC
        assert_eq!(unix_time_from_arma_date([2026, 1, 1, 0, 0]), 1_767_225_600);
        // unix epoch itself
        assert_eq!(unix_time_from_arma_date([1970, 1, 1, 0, 0]), 0);
    }
}
