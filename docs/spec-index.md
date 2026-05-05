# Trigram Registry

| Trigram | Spec Name                   | Description                                                                                    | Status   |
| ------- | --------------------------- | ---------------------------------------------------------------------------------------------- | -------- |
| TRX     | Financial Asset Transaction | recording purchases and managing holdings                                                      | active   |
| ACD     | Account Details             | Account holdings, performance and asset list                                                   | active   |
| ACC     | Account Management          | CRUD operations for financial accounts                                                         | active   |
| TXL     | Transaction List            | Browse, edit and delete transactions per (account, asset) pair                                 | active   |
| SEL     | Sell Transaction            | Recording asset sales, holding reduction, and realized P&L                                     | active   |
| MKT     | Market Price                | Manual market price entry per asset; unrealized P&L display                                    | active   |
| AST     | Asset                       | Asset CRUD, archival, and listing                                                              | active   |
| CAT     | Category                    | User-defined asset category management                                                         | active   |
| UPD     | Application Update          | Auto-update detection, download, and install                                                   | active   |
| WEB     | Asset Web Lookup            | Search OpenFIGI to pre-fill the Add Asset form                                                 | active   |
| PFD     | Portfolio Dashboard         | Cross-account aggregate view: KPIs + per-account list (paused — blocked on cash-tracking spec) | planning |
| CSH     | Cash Tracking               | Cash-as-Holding (one per currency); Deposit/Withdrawal; Buy/Sell re-linked to cash             | active   |

---

## Spec granularity rule

Two tiers — pick the one that fits:

**Domain spec** — for pure CRUD within a single bounded context (e.g. `asset.md`, `account.md`). Use when the feature touches one domain and does not extend another use case.

**Feature spec** — for cross-cutting capabilities that span ≥ 2 domains or extend an existing use case (e.g. `account-details.md`, `market-price.md`). One spec per user-visible capability; cross-references the domains it touches.

If two operations share a use case and entity, they belong in the same spec under the same TRIGRAM — even if they appear as separate UI flows (e.g. buy modal vs sell modal).

```
## Entity definition
## Operation A rules   (TRX-010 … TRX-040)
## Operation B rules   (TRX-041 … TRX-080)
```

> **Historical exception:** TRX and SEL are split because SEL was added as an extension spec after TRX was stable. Rule numbers are permanent — do not merge or renumber. The `record_transaction` contract references both. All future transaction-domain rules go into TRX.
