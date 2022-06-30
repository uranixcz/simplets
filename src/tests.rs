use super::{Outcome, User};

fn new_user(id: i64, credit: i64, payments_in: u64, payments_out: u64) -> User {
    User {
        id,
        name: "".to_string(),
        credit,
        payments_in,
        payments_out,
        password: "".to_string(),
        created: "".to_string(),
        permission: 0
    }
}

#[test]
fn payment_limit1() {
    let payer = new_user(0, 10, 1, 0);
    assert_eq!(payer.send_limit(), 424);
    let u2 = new_user(1, 0, 0, 0);
    assert_eq!(payer.payment_limit(&u2), Outcome::PaymentSendLimit(424));
}
#[test]
fn payment_limit2() {
    let payer = new_user(0, 3000, 0, 0);
    let u2 = new_user(1, 0, 0, 0);
    assert_eq!(payer.payment_limit(&u2), Outcome::PaymentReceiveLimit(2500));
}
#[test]
fn payment_limit3() {
    let payer = new_user(0, 10000, 3, 3);
    let u2 = new_user(1, -100, 2, 2);
    assert_eq!(payer.payment_limit(&u2), Outcome::PaymentReceiveLimit(4430));
}
#[test]
fn held_credit_over_limit() {
    let user = new_user(0, 10000, 0, 0);
    assert_eq!(user.receive_limit(), -7500);
}