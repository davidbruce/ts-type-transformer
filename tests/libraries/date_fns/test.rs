// date-fns from Rust, in Node: dates made, moved, compared and formatted.
// Every check! must hold.
#![no_main]
use bindings::*;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // A Friday, local midnight: what parseISO gives for a date alone.
    let friday: Date = parse_iso::<Date>("2024-03-15", None);
    check!(format_date(&friday, "yyyy-MM-dd") == "2024-03-15");
    check!(format_date(&friday, "EEEE") == "Friday");
    check!(get_year(&friday) == 2024.0);
    check!(!is_weekend(&friday));

    // Moved by days: a new Date, the old one unchanged. A date where a
    // generic `DateArg<DateType>` is wanted is its `DateType` case.
    let day = |d: &Date| DateArg::DateType(d.clone());
    let monday: Date = add_days::<Date, Date>(day(&friday), 3.0, None);
    check!(format_date(&monday, "yyyy-MM-dd") == "2024-03-18");
    check!(format_date(&friday, "yyyy-MM-dd") == "2024-03-15");
    check!(is_weekend_with(
        add_days::<Date, Date>(day(&friday), 1.0, None),
        None
    ));
    let first: Date = start_of_month::<Date, Date>(day(&friday), None);
    check!(format_date(&first, "d MMM yyyy") == "1 Mar 2024");

    // Compared: dates, ISO strings and timestamps alike.
    check!(difference_in_days(&monday, &friday) == 3.0);
    check!(is_after(&monday, "2024-03-15"));
    check!(compare_asc(&friday, monday.time()) == -1.0);
    check!(format_distance(&monday, &friday) == "3 days");
    let latest: Date = max::<Date, Date>(vec![day(&friday), day(&monday)], None);
    check!(latest.time() == monday.time());

    // An interval, an options object made here, as a duration.
    let span: Interval<DateArg<Date>, DateArg<Date>> = Interval::create();
    span.set_start(day(&friday));
    span.set_end(day(&add_days::<Date, Date>(day(&friday), 10.0, None)));
    let length = interval_to_duration(&span);
    check!(length.days() == Some(10.0));
    check!(format_duration(&length) == "10 days");

    // A duration made here, added.
    let later = Duration::create();
    later.set_months(Some(1.0));
    later.set_days(Some(2.0));
    let moved: Date = add::<Date, Date>(day(&friday), &later, None);
    check!(format_date(&moved, "yyyy-MM-dd") == "2024-04-17");

    // Nonsense is no date.
    check!(!is_valid(&parse_iso::<Date>("not a date", None)));
}
