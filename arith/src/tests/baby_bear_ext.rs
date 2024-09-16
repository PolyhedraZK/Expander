use super::{extension_field::random_extension_field_tests, field::random_field_tests};
use crate::BabyBearExt4;

#[test]
fn test_field() {
    random_field_tests::<BabyBearExt4>("Baby Bear Ext4".to_string());
    random_extension_field_tests::<BabyBearExt4>("Baby Bear Ext4".to_string());
}
