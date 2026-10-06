//! Envío de bytes crudos (ESC/POS) a la impresora térmica.
//!
//! INTENTO ANTERIOR (descartado): abrir `\\.\USB001` directo como archivo.
//! No funcionó porque ese nombre de puerto solo existe una vez que Windows
//! lo asocia a una cola de impresión — no se crea solo por conectar el USB.
//!
//! ENFOQUE ACTUAL: igual que cualquier impresora normal en Windows, pero
//! usando el driver "Generic / Text Only" (que no reformatea nada) y
//! mandando los bytes con tipo de dato "RAW" vía la API del spooler
//! (winspool.drv). Esto es lo mismo que hacen la mayoría de integraciones
//! de impresoras térmicas en Windows.
//!
//! Requisito: haber creado la cola en Windows una vez (Agregar impresora →
//! puerto USB001 → driver Generic/Text Only) y saber el nombre EXACTO
//! que le pusiste.

/// Nombre exacto de la impresora tal como aparece en
/// Configuración → Impresoras y escáneres.
///
/// TODO: mover esto a app_settings cuando haya tiempo, para no
/// tener que recompilar si cambia el nombre o el equipo.
pub const PRINTER_NAME: &str = "DIG58IIA";

#[cfg(windows)]
mod win {
    use std::ptr::null_mut;
    use winapi::ctypes::c_void;

    use winapi::shared::minwindef::{DWORD, LPDWORD};
    use winapi::shared::ntdef::HANDLE;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::um::winspool::{
        ClosePrinter, EndDocPrinter, EndPagePrinter, OpenPrinterW, StartDocPrinterW,
        StartPagePrinter, WritePrinter, DOC_INFO_1W,
    };

    fn to_wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn print_raw_to_printer(printer_name: &str, data: &[u8]) -> Result<(), String> {
        unsafe {
            let mut handle: HANDLE = null_mut();
            let mut name_wide = to_wide(printer_name);

            if OpenPrinterW(name_wide.as_mut_ptr(), &mut handle, null_mut()) == 0 {
                let error_code = GetLastError();

                return Err(format!(
                    "No se pudo abrir la impresora '{printer_name}'. Código de Windows: {error_code}"
                ));
            }

            let mut doc_name = to_wide("CashFlow-POS Ticket");
            let mut datatype = to_wide("RAW");

            let mut doc_info = DOC_INFO_1W {
                pDocName: doc_name.as_mut_ptr(),
                pOutputFile: null_mut(),
                pDatatype: datatype.as_mut_ptr(),
            };

            let job_id = StartDocPrinterW(handle, 1, &mut doc_info as *mut DOC_INFO_1W as *mut u8);

            if job_id == 0 {
                ClosePrinter(handle);
                return Err("No se pudo iniciar el trabajo de impresión (StartDocPrinter).".into());
            }

            if StartPagePrinter(handle) == 0 {
                EndDocPrinter(handle);
                ClosePrinter(handle);
                return Err("No se pudo iniciar la página (StartPagePrinter).".into());
            }

            let mut written: DWORD = 0;

            let ok = WritePrinter(
                handle,
                data.as_ptr() as *mut c_void,
                data.len() as DWORD,
                &mut written as *mut DWORD as LPDWORD,
            );

            EndPagePrinter(handle);
            EndDocPrinter(handle);
            ClosePrinter(handle);

            if ok == 0 {
                return Err("WritePrinter falló al enviar los bytes.".into());
            }

            if written as usize != data.len() {
                return Err(format!(
                    "Se enviaron solo {} de {} bytes al spooler.",
                    written,
                    data.len()
                ));
            }

            Ok(())
        }
    }
}

#[cfg(windows)]
pub fn print_raw(bytes: &[u8]) -> Result<(), String> {
    win::print_raw_to_printer(PRINTER_NAME, bytes)
}

#[cfg(not(windows))]
pub fn print_raw(_bytes: &[u8]) -> Result<(), String> {
    Err("La impresión térmica solo está implementada para Windows por ahora.".into())
}

/// Ticket mínimo de prueba, útil para verificar que el nombre de impresora
/// y el driver Generic/Text Only funcionan antes de probar con una venta real.
#[allow(dead_code)]
pub fn print_test_ticket() -> Result<(), String> {
    let mut buf = Vec::new();

    buf.extend_from_slice(b"\x1B\x40"); // ESC @ — reset impresora
    buf.extend_from_slice(b"\x1B\x61\x01"); // centrar texto
    buf.extend_from_slice(b"CashFlow-POS\n");
    buf.extend_from_slice(b"Prueba de impresora\n\n");
    buf.extend_from_slice(b"\x1D\x56\x00"); // GS V 0 — corte total de papel

    print_raw(&buf)
}
