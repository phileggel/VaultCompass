#[cfg(test)]
mod tests {
    use super::super::{FetchAccountAssetPricesForDateError, FetchPriceForDateTask};
    use crate::context::account::AccountError;
    use crate::context::asset::AssetError;

    fn json(error: &FetchAccountAssetPricesForDateError) -> serde_json::Value {
        serde_json::to_value(error).expect("serialize")
    }

    #[test]
    fn use_case_variants_serialize_with_code() {
        let invalid =
            FetchAccountAssetPricesForDateError::Failure(FetchPriceForDateTask::InvalidDate);
        let future =
            FetchAccountAssetPricesForDateError::Failure(FetchPriceForDateTask::DateInFuture);
        let unknown =
            FetchAccountAssetPricesForDateError::Failure(FetchPriceForDateTask::UnknownError);
        assert_eq!(json(&invalid), serde_json::json!({ "code": "InvalidDate" }));
        assert_eq!(json(&future), serde_json::json!({ "code": "DateInFuture" }));
        assert_eq!(
            json(&unknown),
            serde_json::json!({ "code": "UnknownError" })
        );
    }

    #[test]
    fn asset_wrapper_flattens_bc_code() {
        let wrapped = FetchAccountAssetPricesForDateError::Asset(AssetError::DatabaseError);
        assert_eq!(
            json(&wrapped),
            serde_json::json!({ "code": "DatabaseError" })
        );
    }

    #[test]
    fn account_wrapper_flattens_bc_code_with_payload() {
        let wrapped = FetchAccountAssetPricesForDateError::Account(AccountError::AccountNotFound {
            account_id: "abc".into(),
        });
        assert_eq!(
            json(&wrapped),
            serde_json::json!({ "code": "AccountNotFound", "account_id": "abc" })
        );
    }
}
