//! Group state snapshot and its rendering into the terse text the decision
//! model is queried with. See docs/context-format.md.

use arma_rs::{FromArma, FromArmaError};
use std::cmp::Ordering;
use std::fmt::Write as _;

/// The group's current waypoint, relative to the leader. `kind` is empty when
/// the group has no waypoint.
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub kind: String,
    pub distance: f32,
    pub bearing: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub damage: f32,
    pub incapacitated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Contact {
    pub category: String,
    pub distance: f32,
    pub bearing: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupSnapshot {
    pub task: Task,
    pub initial_size: u32,
    /// Alive units only.
    pub units: Vec<Unit>,
    /// Current rounds / initial rounds; 0 when the baseline is unknown.
    pub ammo: f32,
    pub contacts: Vec<Contact>,
}

type RawTask = (String, f32, f32);
type RawUnit = (f32, bool);
type RawContact = (String, f32, f32);
type RawSnapshot = (RawTask, u32, Vec<RawUnit>, f32, Vec<RawContact>);

// Tuple parsing instead of the derive: the derive expects SQF key/value maps,
// while the SQF side sends positional arrays to keep the payload small.
impl FromArma for GroupSnapshot {
    fn from_arma(s: String) -> Result<Self, FromArmaError> {
        let ((kind, distance, bearing), initial_size, units, ammo, contacts) =
            RawSnapshot::from_arma(s)?;
        Ok(Self {
            task: Task {
                kind,
                distance,
                bearing,
            },
            initial_size,
            units: units
                .into_iter()
                .map(|(damage, incapacitated)| Unit {
                    damage,
                    incapacitated,
                })
                .collect(),
            ammo,
            contacts: contacts
                .into_iter()
                .map(|(category, distance, bearing)| Contact {
                    category,
                    distance,
                    bearing,
                })
                .collect(),
        })
    }
}

/// Rounded to 50 m so that tick-to-tick jitter doesn't change the text.
fn round_distance(d: f32) -> u32 {
    ((d / 50.0).round() * 50.0).max(0.0) as u32
}

/// Compass callout for an absolute bearing, e.g. 234 -> "southwest".
fn compass(b: f32) -> &'static str {
    const POINTS: [&str; 8] = [
        "north",
        "northeast",
        "east",
        "southeast",
        "south",
        "southwest",
        "west",
        "northwest",
    ];
    POINTS[((b.rem_euclid(360.0) / 45.0).round() as usize) % 8]
}

fn percent(fraction: f32) -> u32 {
    (fraction * 100.0).round().clamp(0.0, 100.0) as u32
}

fn compare(a: f32, b: f32) -> Ordering {
    a.total_cmp(&b)
}

pub fn render(snapshot: &GroupSnapshot, max_contacts: usize) -> String {
    let mut out = String::new();

    if snapshot.task.kind.is_empty() {
        out.push_str("TASK none.");
    } else {
        let _ = write!(
            out,
            "TASK {} {}m {}.",
            snapshot.task.kind.to_uppercase(),
            round_distance(snapshot.task.distance),
            compass(snapshot.task.bearing)
        );
    }

    let alive = snapshot.units.len();
    let wounded = snapshot.units.iter().filter(|u| u.damage > 0.0).count();
    // Average over alive units: a group of survivors at 20% health is in worse
    // shape than casualty counts alone suggest.
    let hp = if alive == 0 {
        0
    } else {
        percent(1.0 - snapshot.units.iter().map(|u| u.damage).sum::<f32>() / alive as f32)
    };
    let _ = write!(
        out,
        " GRP {alive}/{} alive, {wounded} wnd, hp {hp}%.",
        snapshot.initial_size
    );

    let incapacitated = snapshot.units.iter().filter(|u| u.incapacitated).count();
    let dead = (snapshot.initial_size as usize).saturating_sub(alive);
    let _ = write!(out, " CAS {}.", dead + incapacitated);
    let _ = write!(out, " AMMO {}%.", percent(snapshot.ammo));

    let mut contacts: Vec<&Contact> = snapshot.contacts.iter().collect();
    contacts.sort_by(|a, b| compare(a.distance, b.distance));
    let hidden = contacts.len().saturating_sub(max_contacts);
    contacts.truncate(max_contacts);

    if contacts.is_empty() && hidden == 0 {
        out.push_str(" CONTACTS none.");
    } else {
        let list = contacts
            .iter()
            .map(|c| {
                format!(
                    "{} {}m {}",
                    c.category,
                    round_distance(c.distance),
                    compass(c.bearing)
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let _ = write!(out, " CONTACTS {list}");
        if hidden > 0 {
            // No leading space when the list itself is empty (max_contacts = 0).
            let sep = if list.is_empty() { "" } else { " " };
            let _ = write!(out, "{sep}(+{hidden} more)");
        }
        out.push('.');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(damage: f32, incapacitated: bool) -> Unit {
        Unit {
            damage,
            incapacitated,
        }
    }

    fn contact(category: &str, distance: f32, bearing: f32) -> Contact {
        Contact {
            category: category.to_owned(),
            distance,
            bearing,
        }
    }

    fn sample() -> GroupSnapshot {
        let mut units = vec![unit(0.0, false); 4];
        units.push(unit(0.3, false));
        units.push(unit(0.4, true));
        GroupSnapshot {
            task: Task {
                kind: "SAD".into(),
                distance: 418.0,
                bearing: 45.2,
            },
            initial_size: 8,
            units,
            ammo: 0.45,
            contacts: vec![
                contact("inf", 150.0, 10.0),
                contact("inf", 210.0, 12.0),
                contact("armor", 400.0, 300.0),
            ],
        }
    }

    #[test]
    fn renders_expected_shape() {
        // 6 alive, 2 damaged, mean damage 0.7/6 -> hp 88%; 2 dead + 1 incap.
        assert_eq!(
            render(&sample(), 8),
            "TASK SAD 400m northeast. GRP 6/8 alive, 2 wnd, hp 88%. CAS 3. AMMO 45%. \
             CONTACTS inf 150m north, inf 200m north, armor 400m northwest."
        );
    }

    #[test]
    fn caps_contacts_and_counts_the_rest() {
        let mut s = sample();
        s.contacts = (1..=6)
            .map(|i| contact("inf", 100.0 * i as f32, 0.0))
            .collect();
        let text = render(&s, 2);
        assert!(
            text.ends_with("CONTACTS inf 100m north, inf 200m north (+4 more)."),
            "{text}"
        );
    }

    #[test]
    fn sorts_contacts_by_distance_before_capping() {
        let mut s = sample();
        s.contacts = vec![
            contact("armor", 900.0, 0.0),
            contact("inf", 50.0, 0.0),
            contact("air", 300.0, 0.0),
        ];
        let text = render(&s, 2);
        assert!(
            text.ends_with("CONTACTS inf 50m north, air 300m north (+1 more)."),
            "{text}"
        );
    }

    #[test]
    fn rounds_distance_to_50m_and_names_compass_points() {
        let mut s = sample();
        s.contacts = vec![contact("inf", 174.9, 359.7), contact("inf", 24.0, -10.0)];
        let text = render(&s, 8);
        assert!(
            text.contains("CONTACTS inf 0m north, inf 150m north."),
            "{text}"
        );
    }

    #[test]
    fn no_waypoint() {
        let mut s = sample();
        s.task = Task {
            kind: String::new(),
            distance: 0.0,
            bearing: 0.0,
        };
        assert!(render(&s, 8).starts_with("TASK none. GRP"));
    }

    #[test]
    fn no_contacts() {
        let mut s = sample();
        s.contacts.clear();
        assert!(render(&s, 8).ends_with("CONTACTS none."));
    }

    #[test]
    fn all_dead() {
        let mut s = sample();
        s.units.clear();
        let text = render(&s, 8);
        assert!(
            text.contains("GRP 0/8 alive, 0 wnd, hp 0%. CAS 8."),
            "{text}"
        );
    }

    #[test]
    fn zero_max_contacts_reports_all_hidden() {
        let text = render(&sample(), 0);
        assert!(text.ends_with("CONTACTS (+3 more)."), "{text}");
    }

    #[test]
    fn parses_sqf_array() {
        let raw = r#"[["SAD",420.5,45],8,[[0,false],[0.5,true]],0.45,[["inf",150,10]]]"#;
        let s = GroupSnapshot::from_arma(raw.to_owned()).unwrap();
        assert_eq!(s.task.kind, "SAD");
        assert_eq!(s.initial_size, 8);
        assert_eq!(s.units, vec![unit(0.0, false), unit(0.5, true)]);
        assert_eq!(s.contacts, vec![contact("inf", 150.0, 10.0)]);
    }

    #[test]
    fn parses_empty_lists() {
        let raw = r#"[["",0,0],8,[],0,[]]"#;
        let s = GroupSnapshot::from_arma(raw.to_owned()).unwrap();
        assert!(s.units.is_empty() && s.contacts.is_empty());
    }
}
