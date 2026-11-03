//! Loads `ICU_DATA` into V8 and checks that `Intl` and `Temporal` use the new
//! time zone data.

/// `(time zone, expected offset at 2026-12-01T12:00Z)`. Each zone listed after
/// New York changed in 2026 and is wrong in the ICU 77.1 release data.
const OFFSETS: &[(&str, &str)] = &[
    ("America/New_York", "-05:00"),
    ("America/Vancouver", "-07:00"), // tzdata 2026b
    ("America/Edmonton", "-06:00"),  // 2026c
    ("America/Inuvik", "-06:00"),    // 2026d
    ("America/Winnipeg", "-05:00"),  // 2026e
    ("America/Havana", "-04:00"),    // 2026f
];

fn eval(scope: &mut v8::PinScope, source: &str) -> String {
    let source = v8::String::new(scope, source).unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    script.run(scope).unwrap().to_rust_string_lossy(scope)
}

#[test]
fn v8_uses_patched_tzdata() {
    v8::icu::set_common_data_77(icudtl::ICU_DATA).expect("V8 rejected the ICU data");
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();

    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let mut actual = Vec::new();
    for &(time_zone, _) in OFFSETS {
        let intl = eval(
            scope,
            &format!(
                r#"new Intl.DateTimeFormat("en-US", {{
                    timeZone: "{time_zone}",
                    timeZoneName: "longOffset",
                }})
                    .formatToParts(new Date("2026-12-01T12:00:00Z"))
                    .find((part) => part.type === "timeZoneName").value"#
            ),
        );
        let temporal = eval(
            scope,
            &format!(
                r#"Temporal.Instant.from("2026-12-01T12:00:00Z")
                    .toZonedDateTimeISO("{time_zone}").offset"#
            ),
        );
        actual.push((time_zone, intl, temporal));
    }
    let expected: Vec<_> = OFFSETS
        .iter()
        .map(|&(time_zone, offset)| (time_zone, format!("GMT{offset}"), offset.to_owned()))
        .collect();
    assert_eq!(
        actual, expected,
        "(time zone, Intl offset, Temporal offset)"
    );
}
