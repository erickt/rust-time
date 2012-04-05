import libc::{c_char, c_int, c_long, size_t, time_t};
import io::{reader, reader_util};
import result::{result, ok, err, methods};
import std::time;

export
    timespec,
    get_time,
    tm,
    empty_tm,
    now,
    at,
    now_utc,
    at_utc,
    strptime;

#[abi = "cdecl"]
#[nolink]
native mod libtime {
    // FIXME: The i64 values can be passed by-val when #2064 is fixed.
    fn tzset();
    fn gmtime_r(&&sec: time_t, &&result: tm) -> *tm;
    fn localtime_r(&&sec: time_t, &&result: tm) -> *tm;
    fn timegm(&&tm: tm) -> time_t;
    fn mktime(&&tm: tm) -> time_t;
}

#[doc = "A record specifying a time value in seconds and microseconds."]
type timespec = {sec: i64, nsec: i32};

#[doc = "
Returns the current time as a `timespec` containing the seconds and
microseconds since 1970-01-01T00:00:00Z.
"]
fn get_time() -> timespec {
    let {sec, usec} = time::get_time();
    {sec: sec as i64, nsec: usec as i32 * 1000_i32}
}

type tm = {
    tm_sec: c_int, // seconds after the minute [0-60]
    tm_min: c_int, // minutes after the hour [0-59]
    tm_hour: c_int, // hours after midnight [0-23]
    tm_mday: c_int, // days of the month [1-31]
    tm_mon: c_int, // months since January [0-11]
    tm_year: c_int, // years since 1900
    tm_wday: c_int, // days since Sunday [0-6]
    tm_yday: c_int, // days since January 1 [0-365]
    tm_isdst: c_int, // Daylight Savings Time flag
    tm_gmtoff: c_long, // offset from UTC in seconds
    tm_zone: *c_char, // timezone abbreviation
    tm_nsec: i32,
};

fn empty_tm() -> tm {
    {
        tm_sec: 0 as c_int,
        tm_min: 0 as c_int,
        tm_hour: 0 as c_int,
        tm_mday: 0 as c_int,
        tm_mon: 0 as c_int,
        tm_year: 0 as c_int,
        tm_wday: 0 as c_int,
        tm_yday: 0 as c_int,
        tm_isdst: 0 as c_int,
        tm_gmtoff: 0 as c_long,
        tm_zone: ptr::null(),
        tm_nsec: 0_i32,
    }
}

#[doc = "Returns the specified time in UTC"]
fn at_utc(clock: timespec) -> tm {
    let mut sec = clock.sec as time_t;
    let mut tm = empty_tm();
    libtime::tzset();
    libtime::gmtime_r(sec, tm);
    { tm_nsec: clock.nsec with tm }
}

#[doc = "Returns the current time in UTC"]
fn now_utc() -> tm {
    at_utc(get_time())
}

#[doc = "Returns the specified time in the local timezone"]
fn at(clock: timespec) -> tm {
    let mut sec = clock.sec as time_t;
    let mut tm = empty_tm();
    libtime::tzset();
    libtime::localtime_r(sec, tm);
    { tm_nsec: clock.nsec with tm }
}

#[doc = "Returns the current time in the local timezone"]
fn now() -> tm {
    at(get_time())
}

#[doc = "Parses the time from the string according to the format string."]
fn strptime(s: str, format: str) -> result<tm, str> {
    type tm_mut = {
       mut tm_sec: c_int,
       mut tm_min: c_int,
       mut tm_hour: c_int,
       mut tm_mday: c_int,
       mut tm_mon: c_int,
       mut tm_year: c_int,
       mut tm_wday: c_int,
       mut tm_yday: c_int,
       mut tm_isdst: c_int,
       mut tm_gmtoff: c_long,
       mut tm_zone: *c_char,
       mut tm_nsec: i32,
    };

    fn match_str(s: str, pos: uint, needle: str) -> bool {
        let mut i = pos;
        for str::each(needle) {|ch|
            if s[i] != ch {
                ret false;
            }
            i += 1u;
        }
        ret true;
    }

    fn match_strs(s: str, pos: uint, strs: [(str, i32)])
      -> option<(i32, uint)> {
        let mut i = 0u;
        let len = vec::len(strs);
        while i < len {
            let (needle, value) = strs[i];

            if match_str(s, pos, needle) {
                ret some((value, pos + str::len(needle)));
            }
            i += 1u;
        }

        none
    }

    fn match_digits(s: str, pos: uint, digits: uint, ws: bool)
      -> option<(i32, uint)> {
        let mut pos = pos;
        let mut value = 0 as c_int;

        let mut i = 0u;
        while i < digits {
            let {ch, next} = str::char_range_at(s, pos);
            pos = next;

            alt ch {
              '0' to '9' {
                value = value * 10 as c_int + (ch as i32 - '0' as i32);
              }
              ' ' if ws { }
              _ { ret none; }
            }
            i += 1u;
        }

        some((value, pos))
    }

    fn parse_char(s: str, pos: uint, c: char) -> result<uint, str> {
        let {ch, next} = str::char_range_at(s, pos);

        if c == ch {
            ok(next)
        } else {
            err(#fmt("Expected %?, found %?",
                str::from_char(c),
                str::from_char(ch)))
        }
    }

    fn parse_type(s: str, pos: uint, ch: char, tm: tm_mut)
      -> result<uint, str> {
        alt ch {
          'A' {
            alt match_strs(s, pos, [
                ("Sunday", 0 as c_int),
                ("Monday", 1 as c_int),
                ("Tuesday", 2 as c_int),
                ("Wednesday", 3 as c_int),
                ("Thursday", 4 as c_int),
                ("Friday", 5 as c_int),
                ("Saturday", 6 as c_int)
            ]) {
              some(item) { let (v, pos) = item; tm.tm_wday = v; ok(pos) }
              none { err("Invalid day") }
            }
          }
          'a' {
            alt match_strs(s, pos, [
                ("Sun", 0 as c_int),
                ("Mon", 1 as c_int),
                ("Tue", 2 as c_int),
                ("Wed", 3 as c_int),
                ("Thu", 4 as c_int),
                ("Fri", 5 as c_int),
                ("Sat", 6 as c_int)
            ]) {
              some(item) { let (v, pos) = item; tm.tm_wday = v; ok(pos) }
              none { err("Invalid day") }
            }
          }
          'B' {
            alt match_strs(s, pos, [
                ("January", 0 as c_int),
                ("February", 1 as c_int),
                ("March", 2 as c_int),
                ("April", 3 as c_int),
                ("May", 4 as c_int),
                ("June", 5 as c_int),
                ("July", 6 as c_int),
                ("August", 7 as c_int),
                ("September", 8 as c_int),
                ("October", 9 as c_int),
                ("November", 10 as c_int),
                ("December", 11 as c_int)
            ]) {
              some(item) { let (v, pos) = item; tm.tm_mon = v; ok(pos) }
              none { err("Invalid month") }
            }
          }
          'b' | 'h' {
            alt match_strs(s, pos, [
                ("Jan", 0 as c_int),
                ("Feb", 1 as c_int),
                ("Mar", 2 as c_int),
                ("Apr", 3 as c_int),
                ("May", 4 as c_int),
                ("Jun", 5 as c_int),
                ("Jul", 6 as c_int),
                ("Aug", 7 as c_int),
                ("Sep", 8 as c_int),
                ("Oct", 9 as c_int),
                ("Nov", 10 as c_int),
                ("Dec", 11 as c_int)
            ]) {
              some(item) { let (v, pos) = item; tm.tm_mon = v; ok(pos) }
              none { err("Invalid month") }
            }
          }
          'C' {
            alt match_digits(s, pos, 2u, false) {
              some(item) {
                let (v, pos) = item;
                tm.tm_year += (v * 100 as c_int) - 1900 as c_int;
                ok(pos)
              }
              none { err("Invalid year") }
            }
          }
          'c' {
            parse_type(s, pos, 'a', tm)
                .chain { |pos| parse_char(s, pos, ' ') }
                .chain { |pos| parse_type(s, pos, 'b', tm) }
                .chain { |pos| parse_char(s, pos, ' ') }
                .chain { |pos| parse_type(s, pos, 'e', tm) }
                .chain { |pos| parse_char(s, pos, ' ') }
                .chain { |pos| parse_type(s, pos, 'T', tm) }
                .chain { |pos| parse_char(s, pos, ' ') }
                .chain { |pos| parse_type(s, pos, 'Y', tm) }
          }
          'D' | 'x' {
            parse_type(s, pos, 'm', tm)
                .chain { |pos| parse_char(s, pos, '/') }
                .chain { |pos| parse_type(s, pos, 'd', tm) }
                .chain { |pos| parse_char(s, pos, '/') }
                .chain { |pos| parse_type(s, pos, 'y', tm) }
          }
          'd' {
            alt match_digits(s, pos, 2u, false) {
              some(item) { let (v, pos) = item; tm.tm_mday = v; ok(pos) }
              none { err("Invalid day of the month") }
            }
          }
          'e' {
            alt match_digits(s, pos, 2u, true) {
              some(item) { let (v, pos) = item; tm.tm_mday = v; ok(pos) }
              none { err("Invalid day of the month") }
            }
          }
          'F' {
            parse_type(s, pos, 'Y', tm)
                .chain { |pos| parse_char(s, pos, '-') }
                .chain { |pos| parse_type(s, pos, 'm', tm) }
                .chain { |pos| parse_char(s, pos, '-') }
                .chain { |pos| parse_type(s, pos, 'd', tm) }
          }
          'H' {
            // FIXME: range check.
            alt match_digits(s, pos, 2u, false) {
              some(item) { let (v, pos) = item; tm.tm_hour = v; ok(pos) }
              none { err("Invalid hour") }
            }
          }
          'I' {
            // FIXME: range check.
            alt match_digits(s, pos, 2u, false) {
              some(item) {
                  let (v, pos) = item;
                  tm.tm_hour = if v == 12 as c_int { 0 as c_int } else { v };
                  ok(pos)
              }
              none { err("Invalid hour") }
            }
          }
          'j' {
            // FIXME: range check.
            alt match_digits(s, pos, 3u, false) {
              some(item) {
                let (v, pos) = item;
                tm.tm_yday = v - 1 as c_int;
                ok(pos)
              }
              none { err("Invalid year") }
            }
          }
          'k' {
            // FIXME: range check.
            alt match_digits(s, pos, 2u, true) {
              some(item) { let (v, pos) = item; tm.tm_hour = v; ok(pos) }
              none { err("Invalid hour") }
            }
          }
          'l' {
            // FIXME: range check.
            alt match_digits(s, pos, 2u, true) {
              some(item) {
                  let (v, pos) = item;
                  tm.tm_hour = if v == 12 as c_int { 0 as c_int } else { v };
                  ok(pos)
              }
              none { err("Invalid hour") }
            }
          }
          'M' {
            // FIXME: range check.
            alt match_digits(s, pos, 2u, false) {
              some(item) { let (v, pos) = item; tm.tm_min = v; ok(pos) }
              none { err("Invalid minute") }
            }
          }
          'm' {
            // FIXME: range check.
            alt match_digits(s, pos, 2u, false) {
              some(item) {
                let (v, pos) = item;
                tm.tm_mon = v - 1 as c_int;
                ok(pos)
              }
              none { err("Invalid month") }
            }
          }
          'n' { parse_char(s, pos, '\n') }
          'P' {
            alt match_strs(s, pos, [("am", 0 as c_int), ("pm", 12 as c_int)]) {
              some(item) { let (v, pos) = item; tm.tm_hour += v; ok(pos) }
              none { err("Invalid hour") }
            }
          }
          'p' {
            alt match_strs(s, pos, [("AM", 0 as c_int), ("PM", 12 as c_int)]) {
              some(item) { let (v, pos) = item; tm.tm_hour += v; ok(pos) }
              none { err("Invalid hour") }
            }
          }
          'R' {
            parse_type(s, pos, 'H', tm)
                .chain { |pos| parse_char(s, pos, ':') }
                .chain { |pos| parse_type(s, pos, 'M', tm) }
          }
          'r' {
            parse_type(s, pos, 'I', tm)
                .chain { |pos| parse_char(s, pos, ':') }
                .chain { |pos| parse_type(s, pos, 'M', tm) }
                .chain { |pos| parse_char(s, pos, ':') }
                .chain { |pos| parse_type(s, pos, 'S', tm) }
                .chain { |pos| parse_char(s, pos, ' ') }
                .chain { |pos| parse_type(s, pos, 'p', tm) }
          }
          'S' {
            // FIXME: range check.
            alt match_digits(s, pos, 2u, false) {
              some(item) {
                let (v, pos) = item;
                tm.tm_sec = v;
                ok(pos)
              }
              none { err("Invalid second") }
            }
          }
          //'s' {}
          'T' | 'X' {
            parse_type(s, pos, 'H', tm)
                .chain { |pos| parse_char(s, pos, ':') }
                .chain { |pos| parse_type(s, pos, 'M', tm) }
                .chain { |pos| parse_char(s, pos, ':') }
                .chain { |pos| parse_type(s, pos, 'S', tm) }
          }
          't' { parse_char(s, pos, '\t') }
          'u' {
            // FIXME: range check.
            alt match_digits(s, pos, 1u, false) {
              some(item) {
                let (v, pos) = item;
                tm.tm_wday = v;
                ok(pos)
              }
              none { err("Invalid weekday") }
            }
          }
          'v' {
            parse_type(s, pos, 'e', tm)
                .chain { |pos| parse_char(s, pos, '-') }
                .chain { |pos| parse_type(s, pos, 'b', tm) }
                .chain { |pos| parse_char(s, pos, '-') }
                .chain { |pos| parse_type(s, pos, 'Y', tm) }
          }
          //'W' {}
          'w' {
            // FIXME: range check.
            alt match_digits(s, pos, 1u, false) {
              some(item) { let (v, pos) = item; tm.tm_wday = v; ok(pos) }
              none { err("Invalid weekday") }
            }
          }
          //'X' {}
          //'x' {}
          'Y' {
            // FIXME: range check.
            alt match_digits(s, pos, 4u, false) {
              some(item) {
                let (v, pos) = item;
                tm.tm_year = v - 1900 as c_int;
                ok(pos)
              }
              none { err("Invalid weekday") }
            }
          }
          'y' {
            // FIXME: range check.
            alt match_digits(s, pos, 2u, false) {
              some(item) {
                let (v, pos) = item;
                tm.tm_year = v - 1900 as c_int;
                ok(pos)
              }
              none { err("Invalid weekday") }
            }
          }
          'Z' {
            if match_str(s, pos, "UTC") || match_str(s, pos, "GMT") {
                tm.tm_gmtoff = 0 as c_long;
                // FIXME: this should be "UTC"
                tm.tm_zone = ptr::null();
                ok(pos + 3u)
            } else {
                // It's odd, but to maintain compatibility with c's
                // strptime we ignore the timezone.
                let mut pos = pos;
                let len = str::len(s);
                while pos < len {
                    let {ch, next} = str::char_range_at(s, pos);
                    pos = next;
                    if ch == ' ' { break; }
                }

                ok(pos)
            }
          }
          'z' {
            let {ch, next} = str::char_range_at(s, pos);

            if ch == '+' || ch == '-' {
                alt match_digits(s, next, 4u, false) {
                  some(item) {
                    let (v, pos) = item;
                    if v == 0 as c_int {
                        tm.tm_gmtoff = 0 as c_long;
                        // FIXME: this should be UTC
                        tm.tm_zone = ptr::null();
                    }

                    ok(pos)
                  }
                  none { err("Invalid zone offset") }
                }
            } else {
                err("Invalid zone offset")
            }
          }
          '%' { parse_char(s, pos, '%') }
          ch {
            err(#fmt("unknown formatting type: %?", str::from_char(ch)))
          }
        }
    }

    io::with_str_reader(format) { |rdr|
        let tm = {
            mut tm_sec: 0 as c_int,
            mut tm_min: 0 as c_int,
            mut tm_hour: 0 as c_int,
            mut tm_mday: 0 as c_int,
            mut tm_mon: 0 as c_int,
            mut tm_year: 0 as c_int,
            mut tm_wday: 0 as c_int,
            mut tm_yday: 0 as c_int,
            mut tm_isdst: 0 as c_int,
            mut tm_gmtoff: 0 as c_long,
            mut tm_zone: ptr::null(),
            mut tm_nsec: 0i32,
        };
        let mut pos = 0u;
        let len = str::len(s);
        let mut result = err("Invalid time");

        while !rdr.eof() && pos < len {
            let {ch, next} = str::char_range_at(s, pos);

            alt rdr.read_char() {
              '%' {
                alt parse_type(s, pos, rdr.read_char(), tm) {
                  ok(next) { pos = next; }
                  err(e) { result = err(e); break; }
                }
              }
              c {
                if c != ch { break }
                pos = next;
              }
            }
        }

        if pos == len && rdr.eof() {
            ok({
                tm_sec: tm.tm_sec,
                tm_min: tm.tm_min,
                tm_hour: tm.tm_hour,
                tm_mday: tm.tm_mday,
                tm_mon: tm.tm_mon,
                tm_year: tm.tm_year,
                tm_wday: tm.tm_wday,
                tm_yday: tm.tm_yday,
                tm_isdst: tm.tm_isdst,
                tm_gmtoff: tm.tm_gmtoff,
                tm_zone: tm.tm_zone,
                tm_nsec: tm.tm_nsec,
            })
        } else { result }
    }
}

fn strftime(format: str, tm: tm) -> str {
    fn parse_type(ch: char, tm: tm) -> str {
        //FIXME: Implement missing types.
        alt check ch {
          'A' {
            alt check tm.tm_wday as int {
              0 { "Sunday" }
              1 { "Monday" }
              2 { "Tuesday" }
              3 { "Wednesday" }
              4 { "Thursday" }
              5 { "Friday" }
              6 { "Saturday" }
            }
          }
          'a' {
            alt check tm.tm_wday as int {
              0 { "Sun" }
              1 { "Mon" }
              2 { "Tue" }
              3 { "Wed" }
              4 { "Thu" }
              5 { "Fri" }
              6 { "Sat" }
            }
          }
          'B' {
            alt check tm.tm_mon as int {
              0 { "January" }
              1 { "February" }
              2 { "March" }
              3 { "April" }
              4 { "May" }
              5 { "June" }
              6 { "July" }
              7 { "August" }
              8 { "September" }
              9 { "October" }
              10 { "November" }
              11 { "December" }
            }
          }
          'b' | 'h' {
            alt check tm.tm_mon as int {
              0 { "Jan" }
              1 { "Feb" }
              2 { "Mar" }
              3 { "Apr" }
              4 { "May" }
              5 { "Jun" }
              6 { "Jul" }
              7 { "Aug" }
              8 { "Sep" }
              9 { "Oct" }
              10 { "Nov" }
              11 { "Dec" }
            }
          }
          'C' { #fmt("%02d", (tm.tm_year as int + 1900) / 100) }
          'c' {
            #fmt("%s %s %s %s %s",
                parse_type('a', tm),
                parse_type('b', tm),
                parse_type('e', tm),
                parse_type('T', tm),
                parse_type('Y', tm))
          }
          'D' | 'x' {
            #fmt("%s/%s/%s",
                parse_type('m', tm),
                parse_type('d', tm),
                parse_type('y', tm))
          }
          'd' { #fmt("%02d", tm.tm_mday as int) }
          'e' { #fmt("%2d", tm.tm_mday as int) }
          'F' {
            #fmt("%s-%s-%s",
                parse_type('Y', tm),
                parse_type('m', tm),
                parse_type('d', tm))
          }
          //'G' {}
          //'g' {}
          'H' { #fmt("%02d", tm.tm_hour as int) }
          'I' {
            let mut h = tm.tm_hour as int;
            if h == 0 { h = 12 }
            if h > 12 { h -= 12 }
            #fmt("%02d", h)
          }
          'j' { #fmt("%03d", tm.tm_yday as int + 1) }
          'k' { #fmt("%2d", tm.tm_hour as int) }
          'l' {
            let mut h = tm.tm_hour as int;
            if h == 0 { h = 12 }
            if h > 12 { h -= 12 }
            #fmt("%2d", h)
          }
          'M' { #fmt("%02d", tm.tm_min as int) }
          'm' { #fmt("%02d", tm.tm_mon as int + 1) }
          'n' { "\n" }
          'P' { if tm.tm_hour as int < 12 { "am" } else { "pm" } }
          'p' { if tm.tm_hour as int < 12 { "AM" } else { "PM" } }
          'R' {
            #fmt("%s:%s",
                parse_type('H', tm),
                parse_type('M', tm))
          }
          'r' {
            #fmt("%s:%s:%s %s",
                parse_type('I', tm),
                parse_type('M', tm),
                parse_type('S', tm),
                parse_type('p', tm))
          }
          'S' { #fmt("%02d", tm.tm_sec as int) }
          's' { #fmt("%d", tm.to_timespec().sec as int) }
          'T' | 'X' {
            #fmt("%s:%s:%s",
                parse_type('H', tm),
                parse_type('M', tm),
                parse_type('S', tm))
          }
          't' { "\t" }
          //'U' {}
          'u' {
            let i = tm.tm_wday as int;
            int::str(if i == 0 { 7 } else { i })
          }
          //'V' {}
          'v' {
            #fmt("%s-%s-%s",
                parse_type('e', tm),
                parse_type('b', tm),
                parse_type('Y', tm))
          }
          //'W' {}
          'w' { int::str(tm.tm_wday as int) }
          //'X' {}
          //'x' {}
          'Y' { int::str(tm.tm_year as int + 1900) }
          'y' { #fmt("%02d", (tm.tm_year as int + 1900) % 100) }
          'Z' {
            if tm.tm_zone == ptr::null() {
                ""
            } else {
                unsafe { str::unsafe::from_c_str(tm.tm_zone) }
            }
          }
          'z' {
            let gmtoff = tm.tm_gmtoff as i32;
            let sign = if gmtoff > 0_i32 { '+' } else { '-' };
            let mut m = i32::abs(gmtoff) / 60_i32;
            let h = m / 60_i32;
            m -= h * 60_i32;
            #fmt("%c%02d%02d", sign, h as int, m as int)
          }
          //'+' {}
          '%' { "%" }
        }
    }

    let mut buf = "";

    io::with_str_reader(format) { |rdr|
        while !rdr.eof() {
            alt rdr.read_char() {
                '%' { buf += parse_type(rdr.read_char(), tm); }
                ch { str::push_char(buf, ch); }
            }
        }
    }

    buf
}

impl tm for tm {
    #[doc = "Convert time to the seconds from January 1, 1970"]
    fn to_timespec() -> timespec {
        let sec = if self.tm_gmtoff == 0 as c_long {
            libtime::timegm(self) as i64
        } else {
            libtime::mktime(self) as i64
        };

        { sec: sec, nsec: self.tm_nsec }
    }

    #[doc = "Convert time to the local timezone"]
    fn to_local() -> tm {
        at(self.to_timespec())
    }

    #[doc = "Convert time to the UTC"]
    fn to_utc() -> tm {
        at_utc(self.to_timespec())
    }

    #[doc = "
    Return a string of the current time in the form
    \"Thu Jan  1 00:00:00 1970\".
    "]
    fn ctime() -> str { self.strftime("%c") }

    #[doc = "Formats the time according to the format string."]
    fn strftime(format: str) -> str { strftime(format, self) }

    #[doc = "
    Returns a time string formatted according to RFC 822.

    local: \"Thu, 22 Mar 2012 07:53:18 PST\"
    utc:   \"Thu, 22 Mar 2012 14:53:18 UTC\"
    "]
    fn rfc822() -> str {
        if self.tm_gmtoff == 0 as c_long {
            self.strftime("%a, %d %b %Y %T GMT")
        } else {
            self.strftime("%a, %d %b %Y %T %Z")
        }
    }

    #[doc = "
    Returns a time string formatted according to RFC 822 with Zulu time.

    local: \"Thu, 22 Mar 2012 07:53:18 -0700\"
    utc:   \"Thu, 22 Mar 2012 14:53:18 -0000\"
    "]
    fn rfc822z() -> str {
        self.strftime("%a, %d %b %Y %T %z")
    }

    #[doc = "
    Returns a time string formatted according to ISO 8601.

    local: \"2012-02-22T07:53:18-07:00\"
    utc:   \"2012-02-22T14:53:18Z\"
    "]
    fn rfc3339() -> str {
        if self.tm_gmtoff == 0 as c_long{
            self.strftime("%Y-%m-%dT%H:%M:%SZ")
        } else {
            let s = self.strftime("%Y-%m-%dT%H:%M:%S");
            let gmtoff = self.tm_gmtoff as i32;
            let sign = if gmtoff > 0_i32 { '+' } else { '-' };
            let mut m = i32::abs(gmtoff) / 60_i32;
            let h = m / 60_i32;
            m -= h * 60_i32;
            s + #fmt("%c%02d:%02d", sign, h as int, m as int)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_at_utc() {
        os::setenv("TZ", "America/Los_Angeles");

        let time = { sec: 1234567890_i64, nsec: 54321_i32 };
        let utc = at_utc(time);

        assert utc.tm_sec == 30 as c_int;
        assert utc.tm_min == 31 as c_int;
        assert utc.tm_hour == 23 as c_int;
        assert utc.tm_mday == 13 as c_int;
        assert utc.tm_mon == 1 as c_int;
        assert utc.tm_year == 109 as c_int;
        assert utc.tm_wday == 5 as c_int;
        assert utc.tm_yday == 43 as c_int;
        assert utc.tm_isdst == 0 as c_int;
        assert utc.tm_gmtoff == 0 as c_long;
        assert utc.tm_zone != ptr::null();
        assert unsafe { str::unsafe::from_c_str(utc.tm_zone) } == "UTC";
        assert utc.tm_nsec == 54321_i32;
    }

    #[test]
    fn test_at() {
        os::setenv("TZ", "America/Los_Angeles");

        let time = { sec: 1234567890_i64, nsec: 54321_i32 };
        let local = at(time);

        assert local.tm_sec == 30 as c_int;
        assert local.tm_min == 31 as c_int;
        assert local.tm_hour == 15 as c_int;
        assert local.tm_mday == 13 as c_int;
        assert local.tm_mon == 1 as c_int;
        assert local.tm_year == 109 as c_int;
        assert local.tm_wday == 5 as c_int;
        assert local.tm_yday == 43 as c_int;
        assert local.tm_isdst == 0 as c_int;
        assert local.tm_gmtoff == -28800 as c_long;
        assert local.tm_zone != ptr::null();
        assert unsafe { str::unsafe::from_c_str(local.tm_zone) } == "PST";
        assert local.tm_nsec == 54321_i32;
    }

    #[test]
    fn test_to_timespec() {
        os::setenv("TZ", "America/Los_Angeles");

        let time = { sec: 1234567890_i64, nsec: 54321_i32 };
        let utc = at_utc(time);

        assert utc.to_timespec() == time;
        assert utc.to_local().to_timespec() == time;
    }

    #[test]
    fn test_conversions() {
        os::setenv("TZ", "America/Los_Angeles");

        let time = { sec: 1234567890_i64, nsec: 54321_i32 };
        let utc = at_utc(time);
        let local = at(time);

        assert local.to_local() == local;
        assert local.to_utc() == utc;
        assert local.to_utc().to_local() == local;
        assert utc.to_utc() == utc;
        assert utc.to_local() == local;
        assert utc.to_local().to_utc() == utc;
    }

    #[test]
    fn test_strptime() {
        os::setenv("TZ", "America/Los_Angeles");

        alt strptime("", "") {
          ok(tm) {
            assert tm.tm_sec == 0 as c_int;
            assert tm.tm_min == 0 as c_int;
            assert tm.tm_hour == 0 as c_int;
            assert tm.tm_mday == 0 as c_int;
            assert tm.tm_mon == 0 as c_int;
            assert tm.tm_year == 0 as c_int;
            assert tm.tm_wday == 0 as c_int;
            assert tm.tm_isdst== 0 as c_int;
            assert tm.tm_gmtoff == 0 as c_long;
            assert tm.tm_zone == ptr::null();
            assert tm.tm_nsec == 0_i32;
          }
          err(_) {}
        }

        let format = "%a %b %e %T %Y";
        assert strptime("", format) == err("Invalid time");
        assert strptime("Fri Feb 13 15:31:30", format) == err("Invalid time");

        alt strptime("Fri Feb 13 15:31:30 2009", format) {
          err(e) { fail e }
          ok(tm) {
            assert tm.tm_sec == 30 as c_int;
            assert tm.tm_min == 31 as c_int;
            assert tm.tm_hour == 15 as c_int;
            assert tm.tm_mday == 13 as c_int;
            assert tm.tm_mon == 1 as c_int;
            assert tm.tm_year == 109 as c_int;
            assert tm.tm_wday == 5 as c_int;
            assert tm.tm_yday == 0 as c_int;
            assert tm.tm_isdst == 0 as c_int;
            assert tm.tm_gmtoff == 0 as c_long;
            assert tm.tm_zone == ptr::null();
            assert tm.tm_nsec == 0_i32;
          }
        }

        fn test(s: str, format: str) -> bool {
            alt strptime(s, format) {
              ok(tm) { tm.strftime(format) == s }
              err(e) { fail e }
            }
        }

        vec::iter([
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday"
        ]) { |day| assert test(day, "%A"); }

        vec::iter([
            "Sun",
            "Mon",
            "Tue",
            "Wed",
            "Thu",
            "Fri",
            "Sat"
        ]) { |day| assert test(day, "%a"); }

        vec::iter([
            "January",
            "February",
            "March",
            "April",
            "May",
            "June",
            "July",
            "August",
            "September",
            "October",
            "November",
            "December"
        ]) { |day| assert test(day, "%B"); }

        vec::iter([
            "Jan",
            "Feb",
            "Mar",
            "Apr",
            "May",
            "Jun",
            "Jul",
            "Aug",
            "Sep",
            "Oct",
            "Nov",
            "Dec"
        ]) { |day| assert test(day, "%b"); }

        assert test("19", "%C");
        assert test("Fri Feb 13 23:31:30 2009", "%c");
        assert test("02/13/09", "%D");
        assert test("03", "%d");
        assert test("13", "%d");
        assert test(" 3", "%e");
        assert test("13", "%e");
        assert test("2009-02-13", "%F");
        assert test("03", "%H");
        assert test("13", "%H");
        assert test("03", "%I"); // FIXME: flesh out
        assert test("11", "%I"); // FIXME: flesh out
        assert test("044", "%j");
        assert test(" 3", "%k");
        assert test("13", "%k");
        assert test(" 1", "%l");
        assert test("11", "%l");
        assert test("03", "%M");
        assert test("13", "%M");
        assert test("\n", "%n");
        assert test("am", "%P");
        assert test("pm", "%P");
        assert test("AM", "%p");
        assert test("PM", "%p");
        assert test("23:31", "%R");
        assert test("11:31:30 AM", "%r");
        assert test("11:31:30 PM", "%r");
        assert test("03", "%S");
        assert test("13", "%S");
        assert test("15:31:30", "%T");
        assert test("\t", "%t");
        assert test("1", "%u");
        assert test("7", "%u");
        assert test("13-Feb-2009", "%v");
        assert test("0", "%w");
        assert test("6", "%w");
        assert test("2009", "%Y");
        assert test("09", "%y");
        //FIXME
        assert result::get(strptime("UTC", "%Z")).tm_zone == ptr::null();
        assert result::get(strptime("PST", "%Z")).tm_zone == ptr::null();
        assert result::get(strptime("-0000", "%z")).tm_gmtoff == 0 as c_long;
        assert result::get(strptime("-0800", "%z")).tm_gmtoff == 0 as c_long;
        assert test("%", "%%");
    }

    #[test]
    fn test_ctime() {
        os::setenv("TZ", "America/Los_Angeles");

        let time = { sec: 1234567890_i64, nsec: 54321_i32 };
        let utc   = at_utc(time);
        let local = at(time);

        assert utc.ctime()   == "Fri Feb 13 23:31:30 2009";
        assert local.ctime() == "Fri Feb 13 15:31:30 2009";
    }

    #[test]
    fn test_strftime() {
        os::setenv("TZ", "America/Los_Angeles");

        let time = { sec: 1234567890_i64, nsec: 54321_i32 };
        let utc = at_utc(time);
        let local = at(time);

        assert local.strftime("") == "";
        assert local.strftime("%A") == "Friday";
        assert local.strftime("%a") == "Fri";
        assert local.strftime("%B") == "February";
        assert local.strftime("%b") == "Feb";
        assert local.strftime("%C") == "20";
        assert local.strftime("%c") == "Fri Feb 13 15:31:30 2009";
        assert local.strftime("%D") == "02/13/09";
        assert local.strftime("%d") == "13";
        assert local.strftime("%e") == "13";
        assert local.strftime("%F") == "2009-02-13";
        // assert local.strftime("%G") == "2009";
        // assert local.strftime("%g") == "09";
        assert local.strftime("%H") == "15";
        assert local.strftime("%I") == "03";
        assert local.strftime("%j") == "044";
        assert local.strftime("%k") == "15";
        assert local.strftime("%l") == " 3";
        assert local.strftime("%M") == "31";
        assert local.strftime("%m") == "02";
        assert local.strftime("%n") == "\n";
        assert local.strftime("%P") == "pm";
        assert local.strftime("%p") == "PM";
        assert local.strftime("%R") == "15:31";
        assert local.strftime("%r") == "03:31:30 PM";
        assert local.strftime("%S") == "30";
        assert local.strftime("%s") == "1234567890";
        assert local.strftime("%T") == "15:31:30";
        assert local.strftime("%t") == "\t";
        // assert local.strftime("%U") == "06";
        assert local.strftime("%u") == "5";
        // assert local.strftime("%V") == "07";
        assert local.strftime("%v") == "13-Feb-2009";
        // assert local.strftime("%W") == "06";
        assert local.strftime("%w") == "5";
        // handle "%X"
        // handle "%x"
        assert local.strftime("%Y") == "2009";
        assert local.strftime("%y") == "09";

        // FIXME: We should probably standardize on the timezone
        // abbreviation.
        let zone = local.strftime("%Z");
        assert zone == "PST" || zone == "Pacific Standard Time";

        assert local.strftime("%z") == "-0800";
        assert local.strftime("%%") == "%";

        // FIXME: We should probably standardize on the timezone
        // abbreviation.
        let rfc822 = local.rfc822();
        let prefix = "Fri, 13 Feb 2009 15:31:30 ";
        assert rfc822 == prefix + "PST" ||
               rfc822 == prefix + "Pacific Standard Time";

        assert local.ctime() == "Fri Feb 13 15:31:30 2009";
        assert local.rfc822z() == "Fri, 13 Feb 2009 15:31:30 -0800";
        assert local.rfc3339() == "2009-02-13T15:31:30-08:00";

        assert utc.ctime() == "Fri Feb 13 23:31:30 2009";
        assert utc.rfc822() == "Fri, 13 Feb 2009 23:31:30 GMT";
        assert utc.rfc822z() == "Fri, 13 Feb 2009 23:31:30 -0000";
        assert utc.rfc3339() == "2009-02-13T23:31:30Z";
    }
}
