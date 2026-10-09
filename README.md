# CashFlow POS

CashFlow POS is a Windows desktop point-of-sale application for small retail operations.
It combines a React and TypeScript frontend with a Rust/Tauri backend and PostgreSQL
persistence.

The repository is **source-available but proprietary**. It is publicly readable for
inspection and evaluation, but it is not distributed under an OSI-approved open-source
license. See [LICENSE](LICENSE) for the terms that apply to use, copying, modification,
distribution, and commercial exploitation.

## Project Status and Scope

The operational core is implemented across inventory, sales, cash management, printing,
settings, auditing, licensing, and technical diagnostics. Several navigation routes exist
as placeholders and are explicitly listed below. Packaging, distribution, and the final
productization workflow are not documented as complete because they are not established
by the repository.

There is no objective project-completion metric in the codebase. This README therefore
uses capability status rather than an invented percentage or coverage claim.

### Implemented capabilities

- Product categories, products, variants, and internal barcodes.
- Stock entries, stock outs, manual adjustments, initial stock, low-stock queries, and
  inventory valuation.
- Kardex-style inventory movement history.
- Bulk inventory import and export through Excel files.
- Physical stocktakes with review, application, and cancellation flows.
- Sales with line items, payments, taxes, discounts, and sales history.
- Cash sessions, opening balances, cash movements, closing, and reconciliation data.
- Thermal ticket and product-label printing through ESC/POS-related Windows code.
- Business, currency, tax, timezone, ticket, theme, shortcut, and local operator settings.
- Append-only audit events for business operations and selected technical failures.
- Developer Mode with an in-memory event buffer, startup diagnostics, PostgreSQL health
  information, and a live technical event feed.
- Offline purchase and rental license validation.

### Partial or incomplete capabilities

The following routes currently render placeholder screens rather than complete workflows:

- Customers
- Debts
- Suppliers
- Reports

Other known follow-up work includes richer audit exploration, navigation from Developer
Mode to the audit explorer, persistence of the Developer Mode preference across restarts,
and a documented packaging and update process.

## Architecture

```mermaid
flowchart TD
    UI[React and TypeScript UI] --> Router[React Router]
    Router --> Gate[LicenseGate and AppLayout]
    UI -->|Tauri invoke| Commands[Rust Tauri commands]
    Commands --> Handlers[Module handlers]
    Handlers --> Services[Domain services]
    Services --> Repositories[SQLx repositories]
    Repositories --> DB[(PostgreSQL)]
    Commands --> Logging[Tracing and Developer Mode]
    Logging --> Console[Startup panel and CMD feed]
    Logging --> Memory[In-memory developer event buffer]
```

The frontend communicates with Rust through Tauri commands. Backend modules separate
handlers, services, repositories, models, and DTOs where the module requires those
boundaries. SQLx repositories execute typed PostgreSQL queries and migrations. The
application initializes its connection pool and runs pending migrations during startup.

### Frontend structure

- `src/App.tsx`: application providers and startup synchronization.
- `src/routes`: licensed application routes and layout composition.
- `src/pages`: inventory, sales, cash, stocktake, audit, settings, licensing, and
  Developer Mode screens.
- `src/components`: shared layout, UI, and domain components.
- `src/context`: shared application and settings state.
- `src/services`: typed wrappers around Tauri `invoke` calls.
- `src/utils`: preferences, themes, shortcuts, formatting, validation, and Developer Mode
  synchronization.

### Backend structure

- `src-tauri/src/lib.rs`: Tauri application composition and command registration.
- `src-tauri/src/db`: PostgreSQL connection pool, migrations, and health probes.
- `src-tauri/src/modules`: domain modules and their handlers, services, repositories, and
  models.
- `src-tauri/src/logging.rs`: console formatting, credential redaction, startup panel,
  environment filtering, and repeated-warning suppression.
- `src-tauri/migrations`: versioned PostgreSQL schema migrations.

## Domain Modules

| Module | Responsibility | Current status |
| --- | --- | --- |
| `inventory` | Categories, products, variants, stock, imports, exports, and labels | Implemented |
| `sales` | Sales, line items, payments, taxes, discounts, and history | Implemented |
| `cash` | Cash sessions and cash movements | Implemented |
| `stocktake` | Physical counts, review, application, and cancellation | Implemented |
| `billing` | Ticket and label construction/printing | Implemented in the current desktop flow |
| `settings` | Business, currency, tax, ticket, timezone, theme, and shortcut settings | Implemented |
| `audit` | Business events, technical failures, filtering, pagination, and date boundaries | Implemented |
| `licensing` | Offline license state and signature validation | Implemented with UI-level enforcement |
| `developer` | In-memory diagnostics, startup health, and technical event feed | Implemented |
| `customers` | Customer management | Placeholder route |
| `debts` | Debt management | Placeholder route |
| `suppliers` | Supplier management | Placeholder route |
| `reports` | Business reports | Placeholder route |

## Technology Stack

| Technology | Role |
| --- | --- |
| React 19 | Frontend component model |
| TypeScript | Frontend type safety and service contracts |
| Vite 7 | Frontend development server and production bundling |
| React Router 7 | Client-side navigation |
| Tauri 2 | Desktop shell and Rust/JavaScript command boundary |
| Rust 2021 | Backend application and domain logic |
| Tokio | Async runtime |
| SQLx 0.8 | PostgreSQL access, migrations, and compile-time query checking |
| PostgreSQL | Persistent application data |
| Serde and Serde JSON | Serialization and command payloads |
| Tracing | Structured logging and diagnostics |
| Chrono, UUID, rust_decimal | Time, identifiers, and monetary values |
| Calamine, rust_xlsxwriter | Excel import and export |
| Windows APIs and image tooling | Windows printing and ESC/POS support |
| Vitest | Frontend test runner |

## Project Structure

```text
src/                         React application
  components/                Shared and domain UI components
  context/                   Application state
  pages/                     Route-level screens
  routes/                    Router composition
  services/                  Tauri command clients
  utils/                     Frontend utilities
src-tauri/
  migrations/                PostgreSQL migrations 001 through 012
  src/db/                    Database connection and migration startup
  src/modules/               Domain modules
  src/logging.rs             Console and Developer Mode logging
  src/lib.rs                 Tauri setup and command registry
  tests/                     Rust integration tests
scripts/                     License issuer and development helpers
public/                      Static frontend assets
```

## Prerequisites

The repository targets Windows desktop development and requires:

- Windows 10 or Windows 11.
- Node.js and npm. Node.js 20 or newer is recommended.
- Stable Rust and Cargo.
- PostgreSQL accessible to the development machine.
- Microsoft C++ build tools required by Tauri on Windows.
- VS Code, the Tauri extension, and `rust-analyzer` are recommended but not required.

The exact PostgreSQL server version is not hard-coded by the application. The database
schema is managed by SQLx migrations and the configured `DATABASE_URL`.

## Installation and Database Setup

Install frontend dependencies from the repository root:

```powershell
npm install
```

Create `src-tauri/.env` locally. Do not commit it:

```dotenv
DATABASE_URL=postgres://user:password@localhost:5432/pos_db
```

Create the PostgreSQL database before starting the application. From `src-tauri`, the
SQLx CLI can apply migrations and record them in `_sqlx_migrations`:

```powershell
cargo sqlx migrate run --source migrations
```

The application also runs pending migrations during startup through
`sqlx::migrate!("./migrations")`. Because the project uses SQLx query macros, the schema
must be available to the compiler through `DATABASE_URL`, or a valid SQLx offline cache
must be prepared according to the local SQLx workflow.

## Development Commands

Run these commands from the repository root unless noted otherwise:

```powershell
# Frontend development server
npm run dev

# Tauri desktop development flow
npm run tauri dev

# TypeScript validation
npm.cmd exec -- tsc --noEmit

# Production frontend build
npm.cmd run build

# Frontend tests
npm.cmd run test
```

Run these commands from `src-tauri`:

```powershell
# Check formatting without changing files
cargo fmt --check

# Compile the Rust backend
cargo check

# Run Rust unit and integration tests
cargo test
```

If Cargo reports a build-directory lock, close active Tauri development processes before
retrying. This is a local process-management issue, not an application migration.

## Testing and Verification

Rust tests cover domain rules and selected integration flows for audit, inventory,
licensing, logging, Developer Mode, and monetary calculations. The repository also
contains integration-test targets for audit, inventory, and sales. The frontend package
provides a Vitest command and a TypeScript/build validation path.

No CI workflow or coverage configuration was found in the repository. Test counts,
coverage percentages, benchmarks, and release-readiness claims should therefore be
measured in the environment where they are needed rather than inferred from this README.

A useful pre-review check is:

```powershell
# From src-tauri
cargo fmt --check
cargo check
cargo test

# From the repository root
npm.cmd exec -- tsc --noEmit
npm.cmd run build
git diff --check
```

## Security and Sensitive Configuration

- Keep `src-tauri/.env`, database credentials, private signing keys, and customer data
  outside version control.
- The current ignore rules do not explicitly protect every environment-file name; verify
  Git status before publishing and add a local ignore rule for secret files when needed.
- Logging and Developer Mode redact URL credentials before displaying technical messages.
- Developer Mode stores up to 500 temporary events in memory and is not an audit store.
- Business audit events are persisted in PostgreSQL and protected as append-only by
  database triggers.
- No claim is made that the application is hardened against a local user who controls the
  executable, database, or source code.

## Offline Licensing

The licensing flow is local and offline:

1. The application creates or reads an installation identifier in PostgreSQL.
2. A local issuer signs a purchase or rental payload with an RSA private key.
3. The application validates the signature using the public key embedded in the build.
4. Rental licenses carry an expiration date; purchase licenses are perpetual.

Initialize the issuer key pair:

```powershell
node scripts/license-issuer.mjs init
```

Issue a perpetual purchase license:

```powershell
$installationId = "UUID-copied-from-the-application"
$privateKey = "$env:APPDATA\CashFlowPOS\license-issuer\license-private.pem"
node scripts/license-issuer.mjs issue purchase $installationId "Licensee name" $privateKey ".\license.json"
```

The private key remains under `%APPDATA%\CashFlowPOS\license-issuer\` and must not be
committed or distributed. The current license gate is enforced in the user interface.
It is not tamper-proof DRM, and modifying or recompiling the application can bypass a
UI-only gate. Any commercial rights beyond the permissions in [LICENSE](LICENSE) require
a separate written agreement with the rights holder.

## Known Limitations and Roadmap

- Complete the Customers, Debts, Suppliers, and Reports workflows.
- Add richer audit exploration and direct navigation from Developer Mode.
- Decide whether Developer Mode should persist across application restarts.
- Establish a reproducible packaging, signing, update, and release process.
- Add CI and coverage reporting if they become project requirements.
- Review date/time behavior across all modules that expose date filters, not only Audit.

These items are roadmap statements, not claims that the corresponding functionality is
currently available.

## Licensing and Copyright

CashFlow POS is proprietary software and is **not open source under an OSI-approved
license**. The repository may be publicly readable on GitHub, but public visibility does
not grant a general right to use, reproduce, modify, redistribute, sublicense, or
commercially exploit the software.

The repository currently uses a placeholder rights-holder identity in `LICENSE` because
the legally correct copyright holder and year were not established by the source tree.
Replace the placeholders before publication or commercial distribution. Preserve any
rights and exceptions required by applicable law.

## Contributions, Support, and Contact

No public contribution process, support service, or project contact address is defined in
the repository. Do not treat opening an issue or pull request as a grant of permission to
use or redistribute the software. Commercial permissions and exceptions must be agreed in
writing with the applicable rights holder.
