@echo off
cd /d C:\Users\toonk\Downloads\CashFlow-POS\CashFlow-POS

REM ============ PARTE 1: datos y tipos (fases 1 a 3) ============
set OUT=auditoria_1_datos_y_tipos.txt
if exist "%OUT%" del "%OUT%"
call :add src-tauri\migrations\001_categories.sql
call :add src-tauri\migrations\002_products.sql
call :add src-tauri\migrations\003_inventory_movements.sql
call :add src-tauri\migrations\006_sales.sql
call :add src-tauri\migrations\007_movement_reason.sql
call :add src-tauri\migrations\008_barcode_sequence.sql
call :add src-tauri\migrations\009_stocktake.sql
echo.>>"%OUT%"
echo ===== LISTA DE MIGRACIONES =====>>"%OUT%"
dir /b src-tauri\migrations >>"%OUT%"
call :add src-tauri\src\modules\inventory\models.rs
call :add src-tauri\src\modules\inventory\dto.rs
call :add src-tauri\src\modules\stocktake\models.rs
call :add src-tauri\src\modules\stocktake\dto.rs
call :add src\types\index.ts

REM ============ PARTE 2: backend (fases 4 a 7 y 9) ============
set OUT=auditoria_2_backend.txt
if exist "%OUT%" del "%OUT%"
call :add src-tauri\Cargo.toml
call :add src-tauri\capabilities\default.json
call :add src-tauri\src\lib.rs
call :add src-tauri\src\modules\mod.rs
call :add src-tauri\src\errors\app_error.rs
call :add src-tauri\src\errors\mod.rs
call :add src-tauri\src\modules\inventory\mod.rs
call :add src-tauri\src\modules\inventory\repository.rs
call :add src-tauri\src\modules\inventory\service.rs
call :add src-tauri\src\modules\inventory\handlers.rs
call :add src-tauri\src\modules\inventory\router.rs
call :add src-tauri\src\modules\inventory\import.rs
call :add src-tauri\src\modules\inventory\export.rs
call :add src-tauri\src\modules\inventory\barcode.rs
call :add src-tauri\src\modules\inventory\label.rs
call :add src-tauri\src\modules\inventory\kardex.rs
call :add src-tauri\src\modules\stocktake\mod.rs
call :add src-tauri\src\modules\stocktake\repository.rs
call :add src-tauri\src\modules\stocktake\service.rs
call :add src-tauri\src\modules\stocktake\handlers.rs
call :add src-tauri\src\modules\stocktake\router.rs
call :add src-tauri\src\modules\billing\mod.rs
call :add src-tauri\src\modules\billing\printer.rs
call :add src-tauri\src\modules\sales\repository.rs
call :add src-tauri\src\modules\sales\service.rs
echo.>>"%OUT%"
echo ===== TESTS DE RUST =====>>"%OUT%"
dir /s /b src-tauri\tests >>"%OUT%"
for /r src-tauri\tests %%F in (*.rs) do call :add "%%F"

REM ============ PARTE 3: frontend (fases 8 y 9) ============
set OUT=auditoria_3_frontend.txt
if exist "%OUT%" del "%OUT%"
call :add src\services\inventory.service.ts
call :add src\services\stocktake.service.ts
call :add src\utils\format.ts
call :add src\utils\validators.ts
call :add src\utils\movementReasons.ts
call :add src\routes\AppRouter.tsx
call :add src\components\layout\Sidebar.tsx
call :add src\pages\Inventory\Inventory.tsx
call :add src\pages\ProductDetail\ProductDetail.tsx
call :add src\pages\Stocktake\Stocktake.tsx
call :add src\components\inventory\ProductForm.tsx
call :add src\components\inventory\VariantForm.tsx
call :add src\components\inventory\StockMoveForm.tsx
call :add src\components\inventory\KardexTable.tsx
call :add src\components\inventory\StockBadge.tsx
call :add src\components\inventory\StockSummaryCards.tsx
call :add src\components\inventory\ImportInventoryModal.tsx
call :add src\components\inventory\LabelPrintForm.tsx
call :add src\components\ui\Modal.tsx
call :add src\components\ui\Toast.tsx

echo.
echo Listo: auditoria_1_datos_y_tipos.txt, auditoria_2_backend.txt, auditoria_3_frontend.txt
goto :eof

:add
echo.>>"%OUT%"
echo ===== %~1 =====>>"%OUT%"
if exist "%~1" (type "%~1">>"%OUT%") else (echo [ARCHIVO NO ENCONTRADO]>>"%OUT%")
goto :eof