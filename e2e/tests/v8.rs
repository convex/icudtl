//! Loads `ICU_DATA` into V8 and checks that `Intl` and `Temporal` use the new
//! time zone data, and that `Intl` still finds the locale data's zone names.

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

/// `(time zone, expected "en-US" long name at 2026-12-01T12:00Z)`. Long names
/// come from the locale data via `metaZones.res`, so they break when the two
/// come from different CLDR releases.
const LONG_NAMES: &[(&str, &str)] = &[
    ("Pacific/Honolulu", "Hawaii-Aleutian Standard Time"),
    ("America/Adak", "Hawaii-Aleutian Standard Time"),
    // Permanent -07; Unicode's `zoneinfo64` encodes it as daylight time so
    // that CLDR's existing metazones name it.
    ("America/Vancouver", "Pacific Daylight Time"),
];

fn time_zone_name(scope: &mut v8::PinScope, time_zone: &str, style: &str) -> String {
    eval(
        scope,
        &format!(
            r#"new Intl.DateTimeFormat("en-US", {{
                timeZone: "{time_zone}",
                timeZoneName: "{style}",
            }})
                .formatToParts(new Date("2026-12-01T12:00:00Z"))
                .find((part) => part.type === "timeZoneName").value"#
        ),
    )
}

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
        let intl = time_zone_name(scope, time_zone, "longOffset");
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

    let long_names: Vec<_> = LONG_NAMES
        .iter()
        .map(|&(time_zone, _)| (time_zone, time_zone_name(scope, time_zone, "long")))
        .collect();
    let expected: Vec<_> = LONG_NAMES
        .iter()
        .map(|&(time_zone, name)| (time_zone, name.to_owned()))
        .collect();
    assert_eq!(long_names, expected, "(time zone, Intl long name)");

    // Unicode's timezoneTypes.res aliases `Factory` to `Etc/Unknown`, which V8
    // rejects. See "Differences from ICU 77.1" in the README.
    assert_eq!(
        eval(
            scope,
            r#"try {
                new Intl.DateTimeFormat("en-US", { timeZone: "Factory" });
                "ok";
            } catch (e) {
                e.constructor.name;
            }"#,
        ),
        "RangeError",
        "Intl with the Factory time zone"
    );
}
