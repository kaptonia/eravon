mod common;

use common::{error, node, ok, Home, Reply, Scene};

fn send_with(head: Reply, estimate: Reply) -> (i32, String, bool) {
    let h = Home::new("gas");
    let n = node();
    n.set(Scene { head, estimate, ..Scene::default() });
    let r = h.send(&n);
    let sent = n.asked("eth_sendRawTransaction") > 0 || n.asked("eth_getTransactionCount") > 0 || h.record().exists();
    (r.code, r.get("downToken"), sent)
}

fn estimate_says(estimate: Reply) -> (i32, String, bool) {
    send_with(ok("\"0x40\""), estimate)
}

fn the_calls(got: (i32, String, bool)) {
    let (code, why, sent) = got;
    assert_eq!((code, why.as_str()), (1, "E_CHAIN_EXECUTION"));
    assert!(!sent, "nothing sent");
}

fn not_the_calls(got: (i32, String, bool)) {
    let (code, why, sent) = got;
    assert_eq!(code, 4, "{why}");
    assert!(why == "E_CHAIN_ENDPOINT" || why == "E_CHAIN_TRANSPORT", "{why}");
    assert!(!sent, "nothing sent");
}

fn coded(code: &str, words: &str) -> Reply {
    error(200, &format!("{{\"code\":{code},\"message\":\"{words}\"}}"))
}

macro_rules! marked {
    ($($name:ident: $said:literal => $family:ident;)*) => {$(
        #[test]
        fn $name() {
            $family(estimate_says(coded("-32000", $said)));
        }
    )*};
}

marked! {
    funds_words_are_the_calls: "insufficient funds for gas * price + value" => the_calls;
    a_used_nonce_is_the_calls: "nonce too low" => the_calls;
    pending_words_are_the_calls: "already known" => the_calls;
    underpriced_words_are_the_calls: "replacement transaction underpriced" => the_calls;
    gas_too_low_words_are_the_calls: "intrinsic gas too low" => the_calls;
    revert_words_are_the_calls: "execution reverted" => the_calls;
    rate_limit_words_are_not_the_calls: "daily request limit exceeded" => not_the_calls;
    missing_method_words_are_not_the_calls: "the method eth_estimateGas does not exist/is not available" => not_the_calls;
    credentials_words_are_not_the_calls: "unauthorized: bad api key" => not_the_calls;
    wrong_chain_words_are_not_the_calls: "invalid chain id for signer" => not_the_calls;
}

#[test]
fn code_three_with_data_is_the_calls() {
    the_calls(estimate_says(error(200, "{\"code\":3,\"data\":\"0x\",\"message\":\"x\"}")));
}

#[test]
fn code_three_without_data_is_the_calls() {
    the_calls(estimate_says(coded("3", "x")));
}

#[test]
fn another_code_with_a_reverts_data_is_the_calls() {
    the_calls(estimate_says(error(200, "{\"code\":-32000,\"data\":\"0x08c379a0\",\"message\":\"VM Exception\"}")));
}

#[test]
fn words_not_known_under_a_code_are_the_calls() {
    the_calls(estimate_says(coded("-32000", "gas required exceeds allowance (30000000)")));
}

#[test]
fn data_that_is_not_hex_under_a_code_is_the_calls() {
    the_calls(estimate_says(error(200, "{\"code\":-32000,\"data\":\"see the logs\",\"message\":\"x\"}")));
}

#[test]
fn an_internal_error_code_is_the_calls() {
    the_calls(estimate_says(coded("-32603", "internal error")));
}

#[test]
fn the_neighbours_of_the_rate_limit_code_are_the_calls() {
    for code in ["-32006", "-32004"] {
        the_calls(estimate_says(coded(code, "x")));
    }
}

#[test]
fn a_code_past_64_bits_is_the_calls() {
    the_calls(estimate_says(coded("9223372036854775808", "x")));
}

#[test]
fn a_code_written_as_a_fraction_is_the_calls() {
    the_calls(estimate_says(coded("-32601.0", "x")));
}

#[test]
fn the_method_code_is_not_the_calls() {
    not_the_calls(estimate_says(coded("-32601", "x")));
}

#[test]
fn the_rate_limit_code_is_not_the_calls() {
    not_the_calls(estimate_says(coded("-32005", "x")));
}

#[test]
fn a_coded_error_under_a_server_error_status_is_the_calls() {
    the_calls(estimate_says(error(500, "{\"code\":-32000,\"message\":\"gas required exceeds allowance\"}")));
}

#[test]
fn a_code_written_as_text_is_not_the_calls() {
    not_the_calls(estimate_says(error(200, "{\"code\":\"-32000\",\"message\":\"x\"}")));
}

#[test]
fn a_message_without_a_code_is_not_the_calls() {
    not_the_calls(estimate_says(error(200, "{\"message\":\"upstream unavailable\"}")));
}

#[test]
fn an_error_that_is_bare_text_is_not_the_calls() {
    not_the_calls(estimate_says(error(200, "\"upstream unavailable\"")));
}

#[test]
fn an_answer_that_is_not_json_is_not_the_calls() {
    not_the_calls(estimate_says(Reply::Say(200, "<html>bad gateway</html>".into())));
}

#[test]
fn a_server_error_page_is_not_the_calls() {
    not_the_calls(estimate_says(Reply::Say(502, "<html>bad gateway</html>".into())));
}

#[test]
fn a_rate_limit_page_is_not_the_calls() {
    not_the_calls(estimate_says(Reply::Say(429, "<html>slow down</html>".into())));
}

#[test]
fn a_forbidden_page_is_not_the_calls() {
    not_the_calls(estimate_says(Reply::Say(403, "<html>no</html>".into())));
}

#[test]
fn a_closed_connection_is_not_the_calls() {
    not_the_calls(estimate_says(Reply::Close));
}

#[test]
fn a_head_refused_with_a_code_is_not_the_calls() {
    not_the_calls(send_with(coded("-32000", "gas required exceeds allowance"), ok("\"0x5208\"")));
}
