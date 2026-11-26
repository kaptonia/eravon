use crate::seam;

pub type Verdict = Result<(), &'static str>;

pub fn twice(a: &str, b: &str) -> Verdict {
    if a != b {
        return Err(seam::RULE_MATCH);
    }
    Ok(())
}

pub const FILE_PASSCODE_MIN: usize = crate::backup::PASSWORD_MIN;

pub fn file_passcode(a: &str, b: &str) -> Verdict {
    if a.chars().count() < FILE_PASSCODE_MIN {
        return Err(seam::RULE_FILE_LEN);
    }
    if a != b {
        return Err(seam::RULE_MATCH);
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    fn check(p: &str) -> Verdict {
        match crate::keybox::pin_trouble(p) {
            None => Ok(()),
            Some(t) => match crate::vfault::Fault::from(crate::fault::Fault::known(crate::fault::Known::PinShape, t.as_str().to_string())) {
                crate::vfault::Fault::Rule(r) => Err(r),
                other => panic!("a passcode rule is said as a rule: {other:?}"),
            },
        }
    }

    #[test]
    fn the_shape_takes_eight_ascii_letters_or_digits_and_nothing_else() {
        for ok in ["48261537", "abcdefgh", "Ab3dE7gH", "aBcDeFgH", "x1y2z3w4"] {
            assert_eq!(check(ok), Ok(()), "{ok}");
        }
        for bad in ["", "1234567", "123456789", "4826153 ", "4826153!", "四八二六一五三七", "４８２６１５３７", "abcdefg"] {
            assert_eq!(check(bad), Err(seam::RULE_SHAPE), "{bad:?}");
        }
        assert_eq!(twice("Ab3dE7gH", "ab3de7gh"), Err(seam::RULE_MATCH));
    }

    #[test]
    fn all_same_is_refused_for_any_passcode_and_first_after_shape() {
        assert_eq!(check("00000000"), Err(seam::RULE_SAME));
        assert_eq!(check("aaaaaaaa"), Err(seam::RULE_SAME));
        assert_eq!(check("11111111"), Err(seam::RULE_SAME));
        assert_eq!(check("AAAAAAAA"), Err(seam::RULE_SAME));
        assert!(check("AAAAAAAa").is_ok(), "大小写不同即不全同");
    }

    #[test]
    fn runs_and_dates_are_judged_only_for_all_digit_passcodes() {
        assert_eq!(check("12345678"), Err(seam::RULE_RUN), "升号");
        assert_eq!(check("98765432"), Err(seam::RULE_RUN), "降号");
        assert_eq!(check("78901234"), Err(seam::RULE_RUN), "升号回绕");
        assert_eq!(check("21098765"), Err(seam::RULE_RUN), "降号回绕(连号先于日期)");
        assert!(check("abcdefgh").is_ok() && check("1234567a").is_ok(), "带字母的不查连号与日期");
        assert_eq!(check("19900215"), Err(seam::RULE_DATE), "年月日");
        assert_eq!(check("15021990"), Err(seam::RULE_DATE), "日月年");
        assert_eq!(check("02151990"), Err(seam::RULE_DATE), "月日年");
        assert_eq!(check("21991231"), Err(seam::RULE_DATE), "2199 在界内");
        assert!(check("22001231").is_ok(), "2200 在界外");
        assert!(check("07011231").is_ok(), "1231 年在界外");
        assert!(check("18991231").is_ok(), "1899 在界外");
    }

    #[test]
    fn the_leap_year_rule_is_the_gregorian_one() {
        assert_eq!(check("20000229"), Err(seam::RULE_DATE), "2000-02-29 有这一天");
        assert!(check("19000229").is_ok(), "1900-02-29 没有这一天");
        assert!(check("20000230").is_ok());
        assert!(check("20000431").is_ok());
    }

    #[test]
    fn a_repeated_block_is_not_refused() {
        assert!(twice("12341234", "12341234").is_ok());
    }

    #[test]
    fn the_file_passcode_asks_two_things() {
        assert_eq!(file_passcode("short7", "short7"), Err(seam::RULE_FILE_LEN));
        assert_eq!(file_passcode("longenough", "longenoughx"), Err(seam::RULE_MATCH));
        assert!(file_passcode("longenough", "longenough").is_ok());
        assert!(file_passcode("两遍一致的中文口令", "两遍一致的中文口令").is_ok());
    }
}
