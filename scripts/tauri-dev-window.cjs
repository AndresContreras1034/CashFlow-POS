// Abre `tauri dev` en una ventana CMD nueva (solo Windows).
// Cualquier otro comando de tauri, o cualquier otra plataforma,
// se ejecuta igual que antes: passthrough directo al CLI.
const { spawn, spawnSync } = require("node:child_process");
const path = require("node:path");

const root = path.resolve(__dirname, "..");
const isWin = process.platform === "win32";
const args = process.argv.slice(2);

// Cita un argumento solo si contiene espacios o metacaracteres de cmd.
function q(arg) {
  return /[\s&|<>^()]/.test(arg) ? `"${arg.replace(/"/g, '""')}"` : arg;
}

if (isWin && args[0] === "dev") {
  // Ruta relativa: /D fija el directorio de trabajo en la raíz del proyecto.
  const inner = ["node_modules\\.bin\\tauri.cmd", ...args.map(q)].join(" ");
  const line = `start "CashFlow POS - DEV" /D "${root}" cmd /k ${inner}`;

  // El entorno (RUST_LOG, NO_COLOR, ...) se hereda por defecto.
  const child = spawn("cmd.exe", ["/d", "/s", "/c", `"${line}"`], {
    cwd: root,
    detached: true,
    stdio: "ignore",
    windowsVerbatimArguments: true,
  });
  child.unref();
  process.exit(0);
}

// Passthrough: mismo comportamiento que el script anterior "tauri": "tauri".
const bin = path.join(
  root,
  "node_modules",
  ".bin",
  isWin ? "tauri.cmd" : "tauri"
);
const result = spawnSync(q(bin), args.map(q), {
  cwd: process.cwd(),
  stdio: "inherit",
  shell: true, // necesario para ejecutar .cmd en Node recientes
});
process.exit(result.status ?? 1);
