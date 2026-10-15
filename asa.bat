@echo off
chcp 65001 >nul
setlocal
cd /d "%~dp0"
set "ROOT=%CD%"

REM =====================================================================
REM  MODULO 5 - DEUDAS, CLIENTES Y PROVEEDORES
REM  Backend: debts, customers, suppliers (restock, payables, purchase_order, links)
REM  Frontend: paginas + componentes debts/customers/suppliers, utils/debts,
REM            suppliers*, callPanel, openExternal
REM  Foco: saldos derivados, pagos que superan la deuda, anulaciones,
REM        borrado logico, datos personales, validacion de documentos,
REM        cruce con proveedores.
REM  Genera: AUDITORIA_MODULO_5_A.txt ... _E.txt
REM  Orden de pegado: A, B, C, D, E
REM  Ejecutar desde la raiz (CashFlow-POS\CashFlow-POS)
REM =====================================================================

REM ================= PARTE A: GIT + MIGRACIONES + DEUDAS (backend) =================
set "OUT=%ROOT%\AUDITORIA_MODULO_5_A.txt"
> "%OUT%" echo ##### GIT HEAD #####
git rev-parse HEAD >> "%OUT%" 2>&1
>> "%OUT%" echo ##### GIT STATUS --short #####
git status --short >> "%OUT%" 2>&1
>> "%OUT%" echo.
>> "%OUT%" echo ##### GIT LOG debts / customers / suppliers #####
git log --format="%%h %%ad %%s" --date=short -- src-tauri/src/modules/debts src-tauri/src/modules/customers src-tauri/src/modules/suppliers src/components/debts src/components/customers src/components/suppliers src/pages/Debts src/pages/Customers src/pages/Suppliers >> "%OUT%" 2>&1
>> "%OUT%" echo.
>> "%OUT%" echo ##### MIGRACIONES (lista completa) #####
dir /b /on src-tauri\migrations >> "%OUT%" 2>&1
>> "%OUT%" echo.
>> "%OUT%" echo ##### TESTS DE INTEGRACION (lista) #####
dir /b src-tauri\tests >> "%OUT%" 2>&1

REM Backend de deudas completo
for %%f in (
"src-tauri\src\modules\debts\mod.rs"
"src-tauri\src\modules\debts\models.rs"
"src-tauri\src\modules\debts\dto.rs"
"src-tauri\src\modules\debts\catalog.rs"
"src-tauri\src\modules\debts\repository.rs"
"src-tauri\src\modules\debts\service.rs"
"src-tauri\src\modules\debts\handlers.rs"
"src-tauri\src\modules\debts\router.rs"
) do call :add "%%~f"

REM Migraciones que tocan deudas, clientes, proveedores, compras, pagos
for /f "delims=" %%f in ('findstr /s /i /m "debt customer supplier payable purchase_order restock payment" src-tauri\migrations\*.sql 2^>nul') do call :add "%%f"

REM Busquedas dirigidas (deudas)
call :grep "SALDOS / PAGOS (debts)" "balance saldo paid_amount remaining outstanding amount_due total_paid" "src-tauri\src\modules\debts\*.rs"
call :grep "PAGO MAYOR A LA DEUDA / VALIDACION DE MONTO (debts)" "overpay exceeds exceed greater amount.>.0 amount.<=.0 InvalidAmount" "src-tauri\src\modules\debts\*.rs"
call :grep "ANULACION / ESTADO (debts)" "void cancel annul voided cancelled status" "src-tauri\src\modules\debts\*.rs"
call :grep "TRANSACCIONES Y BLOQUEOS (debts)" "begin( commit( rollback FOR.UPDATE transaction Transaction" "src-tauri\src\modules\debts\*.rs"
call :grep "DINERO f64 / REDONDEO (debts)" "f64 f32 round( Money money::" "src-tauri\src\modules\debts\*.rs"
call :grep "UNWRAP/EXPECT/PANIC (debts)" "unwrap() expect( panic! todo! unimplemented!" "src-tauri\src\modules\debts\*.rs"
call :grep "SQL CONCATENADO (debts)" "format!( push_str QueryBuilder" "src-tauri\src\modules\debts\*.rs"

REM ================= PARTE B: CLIENTES + PROVEEDORES (nucleo) =================
set "OUT=%ROOT%\AUDITORIA_MODULO_5_B.txt"
> "%OUT%" echo ##### GIT HEAD #####
git rev-parse HEAD >> "%OUT%" 2>&1
>> "%OUT%" echo ##### GIT STATUS --short #####
git status --short >> "%OUT%" 2>&1

for %%f in (
"src-tauri\src\modules\customers\mod.rs"
"src-tauri\src\modules\customers\models.rs"
"src-tauri\src\modules\customers\dto.rs"
"src-tauri\src\modules\customers\repository.rs"
"src-tauri\src\modules\customers\service.rs"
"src-tauri\src\modules\customers\handlers.rs"
"src-tauri\src\modules\customers\router.rs"
"src-tauri\src\modules\suppliers\mod.rs"
"src-tauri\src\modules\suppliers\models.rs"
"src-tauri\src\modules\suppliers\dto.rs"
"src-tauri\src\modules\suppliers\repository.rs"
"src-tauri\src\modules\suppliers\service.rs"
"src-tauri\src\modules\suppliers\handlers.rs"
"src-tauri\src\modules\suppliers\router.rs"
"src-tauri\src\modules\suppliers\links.rs"
"src-tauri\src\modules\suppliers\payables.rs"
) do call :add "%%~f"

call :grep "VALIDACION DE DOCUMENTOS / CONTACTO (customers, suppliers)" "document nit cedula tax_id rut phone email validate trim is_empty" "src-tauri\src\modules\customers\*.rs src-tauri\src\modules\suppliers\*.rs"
call :grep "BORRADO LOGICO (customers, suppliers, debts)" "deleted_at is_active active archived soft DELETE.FROM" "src-tauri\src\modules\customers\*.rs src-tauri\src\modules\suppliers\*.rs src-tauri\src\modules\debts\*.rs"
call :grep "DATOS PERSONALES EN LOGS (customers, suppliers, debts)" "tracing:: info! warn! error! debug! println! eprintln!" "src-tauri\src\modules\customers\*.rs src-tauri\src\modules\suppliers\*.rs src-tauri\src\modules\debts\*.rs"
call :grep "UNIQUE / DUPLICADOS (customers, suppliers)" "unique duplicate Duplicate 23505 already.exists" "src-tauri\src\modules\customers\*.rs src-tauri\src\modules\suppliers\*.rs"
call :grep "PAGINACION / LIMIT (customers, suppliers, debts)" "LIMIT OFFSET page per_page pagination" "src-tauri\src\modules\customers\*.rs src-tauri\src\modules\suppliers\*.rs src-tauri\src\modules\debts\*.rs"
call :grep "UNWRAP/EXPECT/PANIC (customers, suppliers nucleo)" "unwrap() expect( panic! todo! unimplemented!" "src-tauri\src\modules\customers\*.rs src-tauri\src\modules\suppliers\*.rs"
call :grep "SQL CONCATENADO (customers, suppliers)" "format!( push_str QueryBuilder" "src-tauri\src\modules\customers\*.rs src-tauri\src\modules\suppliers\*.rs"
call :grep "SALDOS / CUENTAS POR PAGAR (suppliers)" "balance saldo paid_amount outstanding amount_due payable total_paid" "src-tauri\src\modules\suppliers\*.rs"

REM ================= PARTE C: ORDENES DE COMPRA, RESTOCK + CONTRATOS + TESTS =================
set "OUT=%ROOT%\AUDITORIA_MODULO_5_C.txt"
> "%OUT%" echo ##### GIT HEAD #####
git rev-parse HEAD >> "%OUT%" 2>&1
>> "%OUT%" echo ##### GIT STATUS --short #####
git status --short >> "%OUT%" 2>&1

for %%f in (
"src-tauri\src\modules\suppliers\purchase_order.rs"
"src-tauri\src\modules\suppliers\restock.rs"
) do call :add "%%~f"

REM Contratos de otros bloques que este modulo usa directamente
for %%f in (
"src-tauri\src\types\money.rs"
"src-tauri\src\types\pagination.rs"
"src-tauri\src\types\mod.rs"
"src-tauri\src\errors\app_error.rs"
"src-tauri\src\middleware\transaction.rs"
"src-tauri\src\middleware\audit.rs"
"src-tauri\src\middleware\auth.rs"
"src-tauri\src\router.rs"
"src-tauri\src\modules\mod.rs"
"src-tauri\src\modules\audit\actor.rs"
"src-tauri\src\modules\audit\service.rs"
"src-tauri\.env.example"
) do call :add "%%~f"

REM Tests de integracion relacionados
for /f "delims=" %%f in ('dir /b src-tauri\tests 2^>nul ^| findstr /i "debt customer supplier payable purchase restock deuda cliente proveedor"') do call :add "src-tauri\tests\%%f"

REM Busquedas dirigidas
call :grep "PURCHASE ORDER: estados, recepcion, stock (suppliers)" "received receive status cancelled stock_movement kardex INSERT.INTO" "src-tauri\src\modules\suppliers\purchase_order.rs src-tauri\src\modules\suppliers\restock.rs"
call :grep "TRANSACCIONES Y BLOQUEOS (suppliers, customers)" "begin( commit( rollback FOR.UPDATE transaction Transaction" "src-tauri\src\modules\suppliers\*.rs src-tauri\src\modules\customers\*.rs"
call :grep "DINERO f64 / REDONDEO (suppliers, customers)" "f64 f32 round( Money money::" "src-tauri\src\modules\suppliers\*.rs src-tauri\src\modules\customers\*.rs"
call :grep "UNWRAP/EXPECT/PANIC (purchase_order, restock)" "unwrap() expect( panic! todo! unimplemented!" "src-tauri\src\modules\suppliers\purchase_order.rs src-tauri\src\modules\suppliers\restock.rs"
call :grep "AUDITORIA EN DEUDAS/CLIENTES/PROVEEDORES (quien registra)" "audit_service AuditService log_event record_audit audit::" "src-tauri\src\modules\debts\*.rs src-tauri\src\modules\customers\*.rs src-tauri\src\modules\suppliers\*.rs"
call :grep "CRUCE CON CAJA / VENTAS (quien crea/paga deudas desde otros modulos)" "debts:: DebtService create_debt customer_id supplier_id cash_movement" "src-tauri\src\modules\sales\*.rs src-tauri\src\modules\cash\*.rs src-tauri\src\modules\inventory\*.rs src-tauri\src\modules\reports\*.rs"
call :grep "ENFORCEMENT DE LICENCIA en estos modulos" "licens ensure_licensed require_license is_licensed" "src-tauri\src\modules\debts\*.rs src-tauri\src\modules\customers\*.rs src-tauri\src\modules\suppliers\*.rs src-tauri\src\router.rs"

REM ================= PARTE D: FRONTEND + VERIFICACIONES TS =================
set "OUT=%ROOT%\AUDITORIA_MODULO_5_D.txt"
> "%OUT%" echo ##### GIT HEAD #####
git rev-parse HEAD >> "%OUT%" 2>&1
>> "%OUT%" echo ##### GIT STATUS --short #####
git status --short >> "%OUT%" 2>&1

for %%f in (
"src\pages\Debts\Debts.tsx"
"src\pages\Customers\Customers.tsx"
"src\pages\Suppliers\Suppliers.tsx"
"src\components\debts\DebtBadges.tsx"
"src\components\debts\DebtDetail.tsx"
"src\components\debts\DebtField.tsx"
"src\components\debts\DebtFilters.tsx"
"src\components\debts\DebtForm.tsx"
"src\components\debts\DebtList.tsx"
"src\components\debts\DebtSummaryCards.tsx"
"src\components\debts\PaymentForm.tsx"
"src\components\customers\CustomerDetail.tsx"
"src\components\customers\CustomerForm.tsx"
"src\components\customers\CustomerPicker.tsx"
"src\components\suppliers\PurchaseOrderForm.tsx"
"src\components\suppliers\SupplierCallCard.tsx"
"src\components\suppliers\SupplierCallPanel.tsx"
"src\components\suppliers\SupplierCategoriesTab.tsx"
"src\components\suppliers\SupplierCsvButton.tsx"
"src\components\suppliers\SupplierDetail.tsx"
"src\components\suppliers\SupplierForm.tsx"
"src\components\suppliers\SupplierLinkForm.tsx"
"src\components\suppliers\SupplierPayablesTab.tsx"
"src\components\suppliers\SupplierProductsTab.tsx"
"src\components\suppliers\SupplierRestockTab.tsx"
"src\components\suppliers\SuppliersSummaryCards.tsx"
) do call :add "%%~f"

call :grep "ABRIR ENLACES EXTERNOS / tel: mailto: whatsapp (frontend)" "openExternal shell open( window.open href= tel: mailto: wa.me whatsapp" "src\*.ts src\*.tsx"
call :grep "dangerouslySetInnerHTML / eval (frontend modulo)" "dangerouslySetInnerHTML innerHTML eval( new.Function" "src\components\debts\*.tsx src\components\customers\*.tsx src\components\suppliers\*.tsx src\pages\Debts\*.tsx src\pages\Customers\*.tsx src\pages\Suppliers\*.tsx"
call :grep "DOBLE CLIC / ESTADOS DE CARGA en pagos y ordenes (frontend)" "disabled= loading submitting saving isSubmitting onSubmit" "src\components\debts\PaymentForm.tsx src\components\debts\DebtForm.tsx src\components\suppliers\PurchaseOrderForm.tsx src\components\customers\CustomerForm.tsx src\components\suppliers\SupplierForm.tsx"
call :grep "DATOS PERSONALES EN console / localStorage (frontend modulo)" "console.log console.error localStorage sessionStorage" "src\components\debts\*.tsx src\components\customers\*.tsx src\components\suppliers\*.tsx src\pages\Debts\*.tsx src\pages\Customers\*.tsx src\pages\Suppliers\*.tsx src\utils\callPanel.ts src\utils\debts.ts src\utils\suppliers.ts"

>> "%OUT%" echo.
>> "%OUT%" echo ##### VERIFICACION: node .\node_modules\typescript\bin\tsc --noEmit #####
node .\node_modules\typescript\bin\tsc --noEmit >> "%OUT%" 2>&1

REM ================= PARTE E: SERVICIOS, TIPOS, UTILS, TESTS TS + VERIFICACIONES =================
set "OUT=%ROOT%\AUDITORIA_MODULO_5_E.txt"
> "%OUT%" echo ##### GIT HEAD #####
git rev-parse HEAD >> "%OUT%" 2>&1

for %%f in (
"src\services\debts.service.ts"
"src\services\debts.service.test.ts"
"src\services\customers.service.ts"
"src\services\suppliers.service.ts"
"src\services\suppliers.service.test.ts"
"src\services\api.ts"
"src\types\debts.ts"
"src\types\customers.ts"
"src\types\suppliers.ts"
"src\types\index.ts"
"src\utils\debts.ts"
"src\utils\suppliers.ts"
"src\utils\suppliers.test.ts"
"src\utils\supplierExport.ts"
"src\utils\supplierExport.test.ts"
"src\utils\supplierForm.ts"
"src\utils\supplierForm.test.ts"
"src\utils\supplierTabs.ts"
"src\utils\supplierTabs.test.ts"
"src\utils\callPanel.ts"
"src\utils\callPanel.test.ts"
"src\utils\openExternal.ts"
"src\utils\openExternal.test.ts"
"src\utils\csv.ts"
"src\utils\format.ts"
"src\utils\validators.ts"
"src-tauri\Cargo.toml"
"src-tauri\build.rs"
"package.json"
) do call :add "%%~f"

>> "%OUT%" echo.
>> "%OUT%" echo ##### VERIFICACION: vitest run (suite completa) #####
node .\node_modules\vitest\vitest.mjs run >> "%OUT%" 2>&1

pushd src-tauri
>> "%OUT%" echo.
>> "%OUT%" echo ##### VERIFICACION: cargo check #####
cargo check >> "%OUT%" 2>&1
>> "%OUT%" echo.
>> "%OUT%" echo ##### VERIFICACION: cargo test --lib debts #####
cargo test --lib debts >> "%OUT%" 2>&1
>> "%OUT%" echo.
>> "%OUT%" echo ##### VERIFICACION: cargo test --lib customers #####
cargo test --lib customers >> "%OUT%" 2>&1
>> "%OUT%" echo.
>> "%OUT%" echo ##### VERIFICACION: cargo test --lib suppliers #####
cargo test --lib suppliers >> "%OUT%" 2>&1
>> "%OUT%" echo.
>> "%OUT%" echo ##### VERIFICACION: cargo test --lib #####
cargo test --lib >> "%OUT%" 2>&1
>> "%OUT%" echo.
>> "%OUT%" echo ##### VERIFICACION: cargo clippy -- -D warnings #####
cargo clippy -- -D warnings >> "%OUT%" 2>&1
popd

echo.
echo ===== TAMANOS GENERADOS =====
for %%f in ("%ROOT%\AUDITORIA_MODULO_5_A.txt" "%ROOT%\AUDITORIA_MODULO_5_B.txt" "%ROOT%\AUDITORIA_MODULO_5_C.txt" "%ROOT%\AUDITORIA_MODULO_5_D.txt" "%ROOT%\AUDITORIA_MODULO_5_E.txt") do if exist "%%~f" echo %%~nxf %%~zf bytes

endlocal
exit /b

:add
>> "%OUT%" echo.
>> "%OUT%" echo ===== %~1 =====
if not exist "%~1" goto :add_missing
type "%~1" >> "%OUT%" 2>&1
exit /b
:add_missing
>> "%OUT%" echo (NO EXISTE: %~1)
exit /b

:grep
>> "%OUT%" echo.
>> "%OUT%" echo ##### GREP: %~1 #####
findstr /s /i /n /r "%~2" %~3 >> "%OUT%" 2>&1
exit /b