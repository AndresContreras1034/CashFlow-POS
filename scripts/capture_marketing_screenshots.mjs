import { spawn } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const OUT_DIR = 'C:/Users/toonk/Downloads/CashFlow-POS/CashFlow-POS/promocion';

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function run() {
  console.log('--- Iniciando generador definitivo de capturas reales para publicidad ---');

  if (!fs.existsSync(OUT_DIR)) {
    fs.mkdirSync(OUT_DIR, { recursive: true });
  }

  const app = spawn('C:/Users/toonk/Downloads/CashFlow-POS/CashFlow-POS/src-tauri/target/release/pos.exe', [], {
    cwd: 'C:/Users/toonk/Downloads/CashFlow-POS/CashFlow-POS/src-tauri',
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9222',
      DATABASE_URL: 'postgresql://postgres:postgres@localhost:5432/pos_db'
    }
  });

  let page = null;
  for (let i = 0; i < 30; i++) {
    await sleep(500);
    try {
      const res = await fetch('http://localhost:9222/json');
      const pages = await res.json();
      page = pages.find(p => p.type === 'page');
      if (page) break;
    } catch {}
  }

  if (!page) {
    console.error('Error al conectar a WebView2');
    app.kill();
    process.exit(1);
  }

  const ws = new WebSocket(page.webSocketDebuggerUrl);
  let msgId = 1;
  const pending = new Map();

  function send(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = msgId++;
      pending.set(id, { resolve, reject });
      ws.send(JSON.stringify({ id, method, params }));
    });
  }

  ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) reject(msg.error);
      else resolve(msg.result);
    }
  };

  await new Promise(r => ws.onopen = r);
  await send('Page.enable');
  await send('Runtime.enable');

  // Dimensiones 1440x900 para fidelidad gráfica y nitidez perfecta
  await send('Emulation.setDeviceMetricsOverride', {
    width: 1440,
    height: 900,
    deviceScaleFactor: 1,
    mobile: false
  });

  // Esperar a que la interfaz React esté lista
  for (let i = 0; i < 20; i++) {
    const res = await send('Runtime.evaluate', {
      expression: 'document.querySelectorAll(".sidebar-nav a").length',
      returnByValue: true
    });
    if (res.result.value > 0) break;
    await sleep(400);
  }

  async function evalJs(code) {
    return send('Runtime.evaluate', {
      expression: code,
      returnByValue: true,
      awaitPromise: true
    });
  }

  async function takeScreenshot(fileName, description) {
    console.log(`\nCapturando [${fileName}]: ${description}...`);
    const shot = await send('Page.captureScreenshot', { format: 'png' });
    const filePath = path.join(OUT_DIR, fileName);
    fs.writeFileSync(filePath, Buffer.from(shot.data, 'base64'));
    console.log(`✓ Guardado: ${fileName} (${(shot.data.length / 1024).toFixed(1)} KB)`);
  }

  // 1. INVENTARIO Y VALORACIÓN (6676 productos, $19.379.891.455 en stock)
  await evalJs('document.querySelector("a[href=\'/inventory\']")?.click()');
  await sleep(1500);
  await takeScreenshot('01_Inventario_Catalogo_y_Valoracion.png', 'Inventario general y valoración monetaria');

  // 2. REPORTES - ASESOR COMERCIAL CON INTELIGENCIA ARTIFICIAL
  await evalJs('document.querySelector("a[href=\'/reports\']")?.click()');
  await sleep(1200);
  await evalJs(`
    const btn = Array.from(document.querySelectorAll("button.reports-preset")).find(b => b.textContent.includes("30 días"));
    if (btn) btn.click();
    const scroller = document.querySelector(".app-main");
    if (scroller) scroller.scrollTop = 0;
  `);
  await sleep(2500);
  await takeScreenshot('02_Reportes_Asesor_Comercial_IA.png', 'Recomendaciones de IA para promociones y márgenes');

  // 2B. REPORTES - DASHBOARD CON GRÁFICOS Y KPIS (Scroll en .app-main)
  await evalJs(`
    const scroller = document.querySelector(".app-main");
    if (scroller) scroller.scrollTop = 580;
  `);
  await sleep(1500);
  await takeScreenshot('02B_Reportes_Dashboard_Graficos_y_KPIs.png', 'Gráficos de ventas, KPIs ejecutivos y métodos de pago');
  await evalJs(`
    const scroller = document.querySelector(".app-main");
    if (scroller) scroller.scrollTop = 0;
  `);

  // 3. REPORTES - RANKING DE PRODUCTOS
  await evalJs('document.getElementById("reports-tab-products")?.click()');
  await sleep(2000);
  await takeScreenshot('03_Reportes_Ranking_Productos.png', 'Ranking de productos top por ingresos y rentabilidad');

  // 4. REPORTES - ANÁLISIS DE EXISTENCIAS Y ROTACIÓN
  await evalJs('document.getElementById("reports-tab-inventory")?.click()');
  await sleep(2000);
  await takeScreenshot('04_Reportes_Analisis_Inventario.png', 'Análisis de rotación de stock y productos obsoletos');

  // 5. VENTAS / TERMINAL POS (CON PRODUCTO CARGADO EN CARRITO)
  await evalJs('document.querySelector("a[href=\'/sales\']")?.click()');
  await sleep(1500);
  await evalJs(`
    const input = document.querySelector(".sales-scan-form input");
    if (input) {
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value").set;
      setter.call(input, "Aretes");
      input.dispatchEvent(new Event("input", { bubbles: true }));
      document.querySelector(".sales-scan-form button[type='submit']")?.click();
    }
  `);
  await sleep(1500);
  await evalJs(`
    const item = document.querySelector(".sales-search-results li");
    if (item) item.click();
  `);
  await sleep(1500);
  await takeScreenshot('05_Terminal_Punto_De_Venta_POS.png', 'Terminal de ventas POS interactivo con carrito y cobro');

  // 6. HISTORIAL DE VENTAS
  await evalJs('document.querySelector("a[href=\'/sales-history\']")?.click()');
  await sleep(1800);
  await takeScreenshot('06_Historial_De_Ventas.png', 'Historial de ventas y comprobantes emitidos');

  // 7. CONTROL DE CAJA
  await evalJs('document.querySelector("a[href=\'/cash\']")?.click()');
  await sleep(1500);
  await takeScreenshot('07_Control_De_Caja_Y_Efectivo.png', 'Arqueo de caja y balance de efectivo en tiempo real');

  // 8. PROVEEDORES Y COMPRAS SUGERIDAS
  await evalJs('document.querySelector("a[href=\'/suppliers\']")?.click()');
  await sleep(1800);
  await takeScreenshot('08_Proveedores_Y_Sugerencias_Compra.png', 'Gestión de proveedores y reposición inteligente de stock');

  // 9. CLIENTES Y DIRECTORIO COMERCIAL
  await evalJs('document.querySelector("a[href=\'/customers\']")?.click()');
  await sleep(2000);
  await takeScreenshot('09_Clientes_Directorio_Comercial.png', 'Directorio de clientes y fidelización');

  // 10. DEUDAS (Esperando carga de cartera completa)
  await evalJs('document.querySelector("a[href=\'/debts\']")?.click()');
  await sleep(3500);
  await takeScreenshot('10_Deudas_Cuentas_Por_Cobrar.png', 'Gestión de cartera, cuentas por cobrar y abonos');

  // 11. AJUSTES Y CONFIGURACIÓN
  await evalJs('document.querySelector("a[href=\'/settings\']")?.click()');
  await sleep(1500);
  await takeScreenshot('11_Ajustes_Y_Configuracion.png', 'Ajustes de tienda, tickets, moneda e impuestos');

  console.log('\n--- ¡Todas las capturas publicitarias completadas exitosamente! ---');
  ws.close();
  app.kill();
  process.exit(0);
}

run().catch(err => {
  console.error('Error:', err);
  process.exit(1);
});
