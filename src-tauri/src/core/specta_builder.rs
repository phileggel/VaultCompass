use crate::{
    context::{account, asset},
    core::{logger, Event},
    use_cases::{account_deletion, account_details, archive_asset, delete_asset, update_checker},
};

/// create the Specta builder for standard and generate_bindings
pub fn create_specta_builder() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new()
        .typ::<update_checker::UpdateInfo>()
        .typ::<asset::Asset>()
        .typ::<asset::AssetCategory>()
        .typ::<asset::AssetClass>()
        .typ::<asset::AssetCommandError>()
        .typ::<asset::CategoryCommandError>()
        .typ::<asset::AssetPriceCommandError>()
        .typ::<archive_asset::ArchiveAssetCommandError>()
        .typ::<delete_asset::DeleteAssetCommandError>()
        .typ::<account::Account>()
        .typ::<account::UpdateFrequency>()
        .typ::<account::Holding>()
        .typ::<account::AccountCommandError>()
        .typ::<account::Transaction>()
        .typ::<account::TransactionType>()
        .typ::<account::BuyHoldingDTO>()
        .typ::<account::SellHoldingDTO>()
        .typ::<account::CorrectTransactionDTO>()
        .typ::<account::TransactionCommandError>()
        .typ::<account_details::HoldingDetail>()
        .typ::<account_details::ClosedHoldingDetail>()
        .typ::<account_details::AccountDetailsResponse>()
        .typ::<account_details::AccountDetailsCommandError>()
        .typ::<account_deletion::AccountDeletionSummary>()
        .typ::<account_deletion::AccountDeletionCommandError>()
        .commands(tauri_specta::collect_commands![
            asset::get_assets,
            asset::get_assets_with_archived,
            asset::add_asset,
            asset::update_asset,
            archive_asset::archive_asset,
            asset::unarchive_asset,
            delete_asset::delete_asset,
            asset::get_categories,
            asset::add_category,
            asset::update_category,
            asset::delete_category,
            asset::record_asset_price,
            account::get_accounts,
            account::add_account,
            account::update_account,
            account::delete_account,
            logger::log_frontend,
            update_checker::check_for_update,
            update_checker::download_update,
            update_checker::install_update,
            account::get_asset_ids_for_account,
            account::buy_holding,
            account::sell_holding,
            account::correct_transaction,
            account::cancel_transaction,
            account::get_transactions,
            account_details::get_account_details,
            account_deletion::get_account_deletion_summary
        ])
        .events(tauri_specta::collect_events![Event])
}
