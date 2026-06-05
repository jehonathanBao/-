#[test]
#[ignore = "skeleton for future order-risk module"]
fn normal_order_is_not_false_positive() {
    panic!("implement after order-risk service is wired");
}

#[test]
#[ignore = "skeleton for future order-risk module"]
fn duplicate_order_emits_one_alert() {
    panic!("assert stable tenant:shop:order:risk_version dedupe key");
}

#[test]
#[ignore = "skeleton for future order-risk module"]
fn shop_a_cannot_read_shop_b_order() {
    panic!("assert tenant/shop/user scoped query returns 403 or empty page");
}

#[test]
#[ignore = "skeleton for future order-risk module"]
fn manual_release_prevents_later_auto_block() {
    panic!("assert manual release precedence over automated rules");
}

#[test]
#[ignore = "skeleton for future order-risk module"]
fn pii_is_not_logged() {
    panic!("capture logs and assert no phone/address/id/full email/token");
}
