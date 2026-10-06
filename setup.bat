@echo off
title CashFlow POS - Setup

echo.
echo ==========================================
echo        CASHFLOW POS - SETUP
echo ==========================================
echo.

echo [1/4] Comprobando PostgreSQL...

if not exist "C:\Program Files\PostgreSQL\18\bin\psql.exe" (
    echo ERROR: PostgreSQL 18 no fue encontrado.
    pause
    exit /b 1
)

echo PostgreSQL encontrado.
echo.

echo [2/4] Comprobando base de datos...

"C:\Program Files\PostgreSQL\18\bin\psql.exe" -U postgres -h localhost -p 5432 -d pos_db -c "SELECT 1;" >nul 2>&1

if errorlevel 1 (
    echo ERROR: No se pudo conectar a pos_db.
    echo Comprueba que PostgreSQL este iniciado.
    pause
    exit /b 1
)

echo Base de datos disponible.
echo.

echo [3/4] Comprobando dependencias de Node...

if not exist "node_modules" (
    echo node_modules no existe. Instalando dependencias...
    call npm install

    if errorlevel 1 (
        echo ERROR: npm install fallo.
        pause
        exit /b 1
    )
) else (
    echo node_modules encontrado.
)

echo.

echo [4/4] Iniciando CashFlow POS...

call npm run tauri dev

pause