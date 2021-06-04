use crate::api::{ApiOverrideOperator, ValidatedPurchaseEnvironment, ValidatedPurchaseStore};
use core::str::Chars;
use nanoserde::{DeJson, DeJsonErr, DeJsonState, SerJson, SerJsonState};

impl SerJson for ApiOverrideOperator {
    fn ser_json(&self, d: usize, s: &mut SerJsonState) {
        (*self as i32).ser_json(d, s);
    }
}

impl DeJson for ApiOverrideOperator {
    fn de_json(state: &mut DeJsonState, input: &mut Chars) -> Result<Self, DeJsonErr> {
        let value: i32 = DeJson::de_json(state, input)?;
        match value {
            0 => Ok(ApiOverrideOperator::NO_OVERRIDE),
            1 => Ok(ApiOverrideOperator::BEST),
            2 => Ok(ApiOverrideOperator::SET),
            3 => Ok(ApiOverrideOperator::INCREMENT),
            4 => Ok(ApiOverrideOperator::DECREMENT),
            // TODO: macro for line number
            _ => Err(DeJsonErr {
                col: 0,
                line: 0,
                msg: "ApiOverrideOperator value out of range".to_owned(),
            }),
        }
    }
}

impl Default for ApiOverrideOperator {
    fn default() -> Self {
        ApiOverrideOperator::NO_OVERRIDE
    }
}

impl SerJson for ValidatedPurchaseEnvironment {
    fn ser_json(&self, d: usize, s: &mut SerJsonState) {
        (*self as i32).ser_json(d, s);
    }
}

impl DeJson for ValidatedPurchaseEnvironment {
    fn de_json(state: &mut DeJsonState, input: &mut Chars) -> Result<Self, DeJsonErr> {
        let value: i32 = DeJson::de_json(state, input)?;
        match value {
            0 => Ok(ValidatedPurchaseEnvironment::UNKNOWN),
            1 => Ok(ValidatedPurchaseEnvironment::SANDBOX),
            2 => Ok(ValidatedPurchaseEnvironment::PRODUCTION),
            // TODO: macro for line number
            _ => Err(DeJsonErr {
                col: 0,
                line: 0,
                msg: "ValidatePurchaseEnvironment value out of range".to_owned(),
            }),
        }
    }
}

impl Default for ValidatedPurchaseEnvironment {
    fn default() -> Self {
        ValidatedPurchaseEnvironment::UNKNOWN
    }
}

impl SerJson for ValidatedPurchaseStore {
    fn ser_json(&self, d: usize, s: &mut SerJsonState) {
        (*self as i32).ser_json(d, s);
    }
}

impl DeJson for ValidatedPurchaseStore {
    fn de_json(state: &mut DeJsonState, input: &mut Chars) -> Result<Self, DeJsonErr> {
        let value: i32 = DeJson::de_json(state, input)?;
        match value {
            0 => Ok(ValidatedPurchaseStore::APPLE_APP_STORE),
            1 => Ok(ValidatedPurchaseStore::GOOGLE_PLAY_STORE),
            2 => Ok(ValidatedPurchaseStore::HUAWEI_APP_GALLERY),
            // TODO: macro for line number
            _ => Err(DeJsonErr {
                col: 0,
                line: 0,
                msg: "ValidatedPurchaseStore value out of range".to_owned(),
            }),
        }
    }
}

impl Default for ValidatedPurchaseStore {
    fn default() -> Self {
        ValidatedPurchaseStore::APPLE_APP_STORE
    }
}
