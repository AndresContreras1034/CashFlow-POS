```bat
@echo off
cd /d "%~dp0"
set "OUT=%~dp0auditoria_ventas_completa.txt"

echo Generando auditoria... espera.

(
echo ============================================================
echo BLOQUE 0 - GIT, MIGRACIONES, TESTS Y DOCS
echo ============================================================
echo.
echo ===== GIT STATUS =====
git status --short
echo.
echo ===== GIT LOG =====
git log --oneline -15
echo.
echo ===== BRANCH =====
git branch -vv
echo.
echo ===== REMOTE =====
git fetch origin
git branch -r --contains 3bd857d
echo.
echo ===== MIGRACIONES =====
dir /b src-tauri\migrations
echo.
echo ===== TESTS =====
dir /b src-tauri\tests
echo.
echo ===== DOCS =====
dir /b docs
echo.
echo ===== ARCHIVO RARO =====
if exist "uritestscash_sales_integration.rs srcutilssalesFilters.ts" (type "uritestscash_sales_integration.rs srcutilssalesFilters.ts") else (echo ARCHIVO NO ENCONTRADO)

echo.
echo ============================================================
echo BLOQUE 1 - BACKEND DE VENTAS
echo ============================================================
for %%F in (mod dto models repository service handlers router) do call :leer "src-tauri\src\modules\sales\%%F.rs"

echo.
echo ============================================================
echo BLOQUE 2 - BACKEND DE CAJA
echo ============================================================
for %%F in (mod dto models repository service handlers router) do call :leer "src-tauri\src\modules\cash\%%F.rs"

echo.
echo ============================================================
echo BLOQUE 3 - FRONTEND DE VENTAS
echo ============================================================
for %%F in (
"src\pages\Sales\Sales.tsx"
"src\components\sales\PaymentPanel.tsx"
"src\components\sales\SaleCart.tsx"
"src\components\sales\SaleItem.tsx"
"src\components\sales\BarcodeInput.tsx"
"src\hooks\useBarcode.ts"
"src\hooks\usePrinter.ts"
"src\services\sales.service.ts"
) do call :leer %%F

echo.
echo ============================================================
echo BLOQUE 4 - HISTORIAL Y CAJA FRONTEND
echo ============================================================
for %%F in (
"src\pages\SalesHistory\SalesHistory.tsx"
"src\utils\salesFilters.ts"
"src\utils\salesFilters.test.ts"
"src\pages\Cash\Cash.tsx"
"src\components\cash\CashSummary.tsx"
"src\services\cash.service.ts"
) do call :leer %%F

echo.
echo ============================================================
echo BLOQUE 5 - TICKET, TESTS RUST Y MIGRACIONES
echo ============================================================
call :leer "src-tauri\src\modules\billing\ticket.rs"

for %%F in (src-tauri\tests\*.rs) do if exist "%%F" call :leer "%%F"

for %%F in (src-tauri\migrations\015* src-tauri\migrations\016* src-tauri\migrations\017* src-tauri\migrations\018* src-tauri\migrations\019* src-tauri\migrations\020* src-tauri\migrations\021*) do if exist "%%F" call :leer "%%F"

echo.
echo ============================================================
echo BLOQUE 6 - TIPOS, TESTS Y PACKAGE
echo ============================================================
call :leer "src\types\index.ts"
call :leer "src\utils\salesFilters.test.ts"
call :leer "package.json"

) > "%OUT%" 2>&1

echo.
echo Auditoria terminada.
echo Archivo: "%OUT%"
start "" notepad "%OUT%"
exit /b

:leer
echo.
echo ============================================================
echo ARCHIVO: %~1
echo ============================================================
if exist "%~1" (
    type "%~1"
) else (
    echo ARCHIVO NO EXISTE
)
exit /b
```