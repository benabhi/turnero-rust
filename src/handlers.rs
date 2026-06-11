use salvo::prelude::*;
use salvo::http::header;
use salvo::http::cookie::Cookie;
use minijinja::{Environment, context};
use rusqlite::Connection;
use chrono::{Local, NaiveDate, Datelike, Timelike};

use crate::models::{Horario, ErroresFormulario, ContextoFormulario, DatosTurno, TurnoDB, ErroresLogin};
use crate::db::obtener_horas_reservadas;
use crate::{Templates, StaticAssets};

// --- UTILERÍA INTERNA: ENTORNO MAESTRO DE TEMPLATES ---

/// Inicializa y compila dinámicamente el entorno de plantillas MiniJinja inyectando los archivos embebidos.
///
/// # Mecanismo de Rust (Tricky Part):
/// 1. `String::from_utf8_lossy`: El binario embebido entrega los archivos como un arreglo de bytes crudos (`&[u8]`).
///    Para transformarlo a texto seguro sin provocar pánicos si existieran caracteres inválidos, se realiza una
///    conversión con pérdida, reemplazando secuencias no válidas por el carácter de reemplazo de Unicode ``.
/// 2. `Box::leak`: MiniJinja exige que los nombres y el contenido de las plantillas tengan un ciclo de vida estático
///    (`&'static str`), lo que significa que deben vivir durante toda la ejecución del programa. Como las plantillas
///    se extraen en tiempo de ejecución desde la macro de empaquetado como strings dinámicos en el Heap (`String`),
///    se utiliza `Box::leak` para transferir el control de la memoria directamente al sistema operativo, evadiendo el
///    recolector de basura nativo (*Borrow Checker*) de Rust mediante una fuga de memoria controlada y segura.
fn crear_entorno_templates() -> Environment<'static> {
    let mut env = Environment::new();

    for file_path in Templates::iter() {
        if let Some(html_file) = Templates::get(&file_path) {
            let html_raw = String::from_utf8_lossy(&html_file.data).into_owned();
            let nombre_template = file_path.trim_end_matches(".html").to_string();

            let leaked_name: &'static str = Box::leak(nombre_template.into_boxed_str());
            let leaked_html: &'static str = Box::leak(html_raw.into_boxed_str());

            let _ = env.add_template(leaked_name, leaked_html);
        }
    }
    env
}

/// Valida de manera básica la estructura sintáctica de una dirección de correo electrónico.
///
/// Realiza un análisis posicional sobre los componentes obligatorios del string para evitar dependencias
/// pesadas de expresiones regulares (`regex`) en el binario final.
fn es_email_valido(email: &str) -> bool {
    if let Some(pos_at) = email.find('@') {
        let despues_at = &email[pos_at + 1..];
        return pos_at > 0 && !despues_at.is_empty() && despues_at.contains('.') && !despues_at.ends_with('.');
    }
    false
}

// --- HANDLERS VISTAS PÚBLICAS ---

/// Renderiza el formulario público web calculando dinámicamente la disponibilidad horaria de los turnos.
///
/// # Mecanismo de Rust (Tricky Part):
/// El atributo `#[handler]` reescribe la firma de la función asíncrona para que implemente de forma nativa el trait
/// `Handler` de Salvo. Las referencias mutables hacia `Request` y `Response` operan bajo el sistema de propiedad exclusivo
/// del framework por cada hilo de petición HTTP asignado por el runtime de Tokio.
#[handler]
pub async fn vista_formulario(req: &mut Request, res: &mut Response) {
    let env = crear_entorno_templates();
    let ahora = Local::now();
    let fecha_hoy_str = ahora.format("%Y-%m-%d").to_string();

    // Extrae la query de la URL (?fecha=YYYY-MM-DD). Si está ausente, inicializa con la fecha actual del servidor.
    let fecha_seleccionada_str = req.queries()
        .get("fecha")
        .map(|s| s.to_string())
        .unwrap_or_else(|| fecha_hoy_str.clone());

    let fecha_parseada = NaiveDate::parse_from_str(&fecha_seleccionada_str, "%Y-%m-%d")
        .unwrap_or_else(|_| ahora.date_naive());

    let es_pasado = fecha_parseada < ahora.date_naive();
    let num_dia_semana = fecha_parseada.weekday().number_from_monday();
    let es_fin_de_semana = num_dia_semana == 6 || num_dia_semana == 7;

    let turnos_reservados = obtener_horas_reservadas(&fecha_seleccionada_str);
    let mut lista_horarios = Vec::new();

    let bloques = vec![
        "08:00", "08:30", "09:00", "09:30", "10:00", "10:30",
        "11:00", "11:30", "12:00", "12:30", "13:00", "13:30", "14:00"
    ];

    for block in bloques {
        let hora_str = block.to_string();

        if es_pasado || es_fin_de_semana {
            lista_horarios.push(Horario {
                hora: hora_str,
                disponible: false,
                mensaje: "(No disponible)".to_string(),
            });
            continue;
        }

        if turnos_reservados.contains(&hora_str) {
            lista_horarios.push(Horario {
                hora: hora_str,
                disponible: false,
                mensaje: "(Reservado)".to_string(),
            });
            continue;
        }

        // Si la consulta corresponde al día de hoy, inhabilita matemáticamente los bloques horarios que ya expiraron.
        if fecha_seleccionada_str == fecha_hoy_str {
            let partes: Vec<&str> = block.split(':').collect();
            let b_hora: u32 = partes[0].parse().unwrap_or(0);
            let b_minuto: u32 = partes[1].parse().unwrap_or(0);

            let hora_actual = ahora.hour();
            let minuto_actual = ahora.minute();

            if b_hora < hora_actual || (b_hora == hora_actual && b_minuto <= minuto_actual) {
                lista_horarios.push(Horario {
                    hora: hora_str,
                    disponible: false,
                    mensaje: "(No disponible)".to_string(),
                });
                continue;
            }
        }

        lista_horarios.push(Horario {
            hora: hora_str,
            disponible: true,
            mensaje: "(Disponible)".to_string(),
        });
    }

    let contexto = ContextoFormulario {
        old_fecha: fecha_seleccionada_str,
        min_fecha: fecha_hoy_str,
        old_hora: "".to_string(),
        old_nombre: "".to_string(),
        old_apellido: "".to_string(),
        old_dni: "".to_string(),
        old_telefono: "".to_string(),
        old_email: "".to_string(),
        horarios: lista_horarios,
        errores: ErroresFormulario::default(),
        es_fin_de_semana,
    };

    if let Ok(tmpl) = env.get_template("formulario") {
        match tmpl.render(context! { contexto }) {
            Ok(html_renderizado) => {
                res.render(Text::Html(html_renderizado));
            }
            Err(e) => {
                println!("Error de renderizado en MiniJinja: {:?}", e);
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
    }
}

/// Procesa la solicitud POST del formulario, sanitiza los campos, valida restricciones de tiempo real e inserta en la base de datos.
#[handler]
pub async fn procesar_turno(req: &mut Request, res: &mut Response) {
    let fecha: String = req.form::<String>("fecha").await.unwrap_or_default();
    let hora: String = req.form::<String>("hora").await.unwrap_or_default();
    let nombre: String = req.form::<String>("nombre").await.unwrap_or_default();
    let apellido: String = req.form::<String>("apellido").await.unwrap_or_default();
    let raw_dni: String = req.form::<String>("dni").await.unwrap_or_default();
    let raw_telefono: String = req.form::<String>("telefono").await.unwrap_or_default();
    let email: String = req.form::<String>("email").await.unwrap_or_default();

    // Sanitización de strings en el Backend.
    let dni = raw_dni.replace('.', "").replace('-', "").replace(' ', "");
    let telefono = raw_telefono.replace('-', "").replace(' ', "");

    let mut errores = ErroresFormulario::default();
    let mut hay_errores = false;

    if fecha.is_empty() { errores.fecha = Some("La fecha de asistencia es obligatoria.".to_string()); hay_errores = true; }
    if hora.is_empty() { errores.hora = Some("Debe seleccionar un bloque horario válido.".to_string()); hay_errores = true; }
    if nombre.is_empty() { errores.nombre = Some("El nombre del solicitante es obligatorio.".to_string()); hay_errores = true; }
    if apellido.is_empty() { errores.apellido = Some("El apellido del solicitante es obligatorio.".to_string()); hay_errores = true; }
    if dni.is_empty() { errores.dni = Some("El número de documento es obligatorio.".to_string()); hay_errores = true; }

    if !email.is_empty() && !es_email_valido(&email) {
        errores.email = Some("El formato del correo electrónico ingresado no es válido.".to_string());
        hay_errores = true;
    }

    if hay_errores {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Json(errores));
        return;
    }

    let datos = DatosTurno { fecha, hora, nombre, apellido, dni, telefono, email };

    // Validación cronológica estricta en tiempo real.
    if let Ok(fecha_p) = NaiveDate::parse_from_str(&datos.fecha, "%Y-%m-%d") {
        let ahora = Local::now();
        let hoy_local = ahora.date_naive();
        let num_dia = fecha_p.weekday().number_from_monday();

        if fecha_p < hoy_local || num_dia == 6 || num_dia == 7 {
            errores.fecha = Some("Fecha inválida: los turnos se asignan únicamente en días hábiles futuros.".to_string());
            res.status_code(StatusCode::BAD_REQUEST);
            res.render(Json(errores));
            return;
        }

        if fecha_p == hoy_local {
            let partes: Vec<&str> = datos.hora.split(':').collect();
            if partes.len() == 2 {
                let b_hora: u32 = partes[0].parse().unwrap_or(0);
                let b_minuto: u32 = partes[1].parse().unwrap_or(0);

                let hora_actual = ahora.hour();
                let minuto_actual = ahora.minute();

                if b_hora < hora_actual || (b_hora == hora_actual && b_minuto <= minuto_actual) {
                    errores.hora = Some("El bloque horario seleccionado ya expiró. Elija uno más tarde.".to_string());
                    res.status_code(StatusCode::BAD_REQUEST);
                    res.render(Json(errores));
                    return;
                }
            }
        }
    }

    match Connection::open("turnos.db") {
        Ok(conn) => {
            let query = "INSERT INTO turnos (fecha, hora, nombre, apellido, dni, telefono, email)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)";

            let resultado = conn.execute(
                query,
                [
                    &datos.fecha, &datos.hora, &datos.nombre, &datos.apellido,
                    &datos.dni, &datos.telefono, &datos.email,
                ],
            );

            match resultado {
                Ok(_) => {
                    println!("Turno guardado en DB: {} {} - {}", datos.nombre, datos.apellido, datos.hora);

                    let env = crear_entorno_templates();

                    if let Ok(tmpl) = env.get_template("respuesta") {
                        match tmpl.render(context! { turno => datos }) {
                            Ok(html_renderizado) => { res.render(Text::Html(html_renderizado)); }
                            Err(e) => {
                                println!("Error de MiniJinja renderizando respuesta: {:?}", e);
                                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        }
                    } else {
                        println!("Error: No se encontró la plantilla 'respuesta' en el entorno.");
                        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                    }
                }
                // Intercepta de manera controlada la violación del índice único de SQLite (Colisión de concurrencia).
                Err(rusqlite::Error::SqliteFailure(_err, Some(msg))) if msg.contains("UNIQUE constraint failed") => {
                    errores.hora = Some("Este horario ya fue reservado por otro ciudadano.".to_string());
                    res.status_code(StatusCode::CONFLICT);
                    res.render(Json(errores));
                }
                Err(e) => {
                    println!("Error SQLite: {:?}", e);
                    res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(_) => { res.status_code(StatusCode::INTERNAL_SERVER_ERROR); }
    }
}

// --- HANDLERS AUTENTICACIÓN ADMIN ---

/// Renderiza la vista visual de acceso restringido para personal oficial.
#[handler]
pub async fn vista_login(_req: &mut Request, res: &mut Response) {
    let env = crear_entorno_templates();
    if let Ok(tmpl) = env.get_template("login") {
        if let Ok(html) = tmpl.render(context! {}) {
            res.render(Text::Html(html));
            return;
        }
    }
    res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
}

/// Valida las credenciales de administración e inyecta la cookie de sesión cifrada en el cliente.
#[handler]
pub async fn procesar_login(req: &mut Request, res: &mut Response) {
    let usuario = req.form::<String>("usuario").await.unwrap_or_default();
    let clave = req.form::<String>("clave").await.unwrap_or_default();

    let mut errores = ErroresLogin::default();
    let mut hay_error = false;

    if usuario != "admin" {
        errores.usuario = Some("Usuario oficial no registrado.".to_string());
        hay_error = true;
    }
    if clave != "123456" {
        errores.clave = Some("Contraseña de seguridad incorrecta.".to_string());
        hay_error = true;
    }

    if hay_error {
        res.status_code(StatusCode::UNAUTHORIZED);
        res.render(Json(errores));
        return;
    }

    // Configuración y serialización de la cookie de control institucional.
    let mut cookie = Cookie::new("admin_session", "autenticado_ok");
    cookie.set_path("/");
    cookie.set_http_only(true); // Bloquea el acceso a la cookie vía JavaScript (Defensa crítica XSS)
    res.add_cookie(cookie);

    res.status_code(StatusCode::OK);
    res.render(Text::Plain("OK"));
}

/// Remueve físicamente la cookie de sesión forzando su expiración instantánea desde el Backend.
#[handler]
pub async fn procesar_logout(_req: &mut Request, res: &mut Response) {
    let mut cookie = Cookie::new("admin_session", "");
    cookie.set_path("/");
    cookie.set_http_only(true);
    cookie.set_max_age(salvo::http::cookie::time::Duration::ZERO); // Invalida el TTL de la cookie en el navegador

    res.add_cookie(cookie);
    res.render(Redirect::found("/admin/login"));
}

// --- HANDLERS PANEL INTERNO (ADMIN) ---

/// Endpoint inteligente encargado de evaluar la sesión y redirigir al panel o al login según corresponda.
#[handler]
pub async fn redireccion_admin(req: &mut Request, res: &mut Response) {
    if let Some(cookie) = req.cookies().get("admin_session") {
        if cookie.value() == "autenticado_ok" {
            res.render(Redirect::found("/admin/turnos"));
            return;
        }
    }
    res.render(Redirect::found("/admin/login"));
}

/// Renderiza el panel de monitoreo y auditoría de solicitudes de antecedentes penales.
#[handler]
pub async fn vista_admin(req: &mut Request, res: &mut Response) {
    let mut autenticado = false;

    if let Some(cookie) = req.cookies().get("admin_session") {
        if cookie.value() == "autenticado_ok" {
            autenticado = true;
        }
    }

    if !autenticado {
        res.render(Redirect::found("/admin/login"));
        return;
    }

    let env = crear_entorno_templates();
    let mut lista_turnos = Vec::new();

    let ver_todo = req.queries().get("historial").map(|v| v == "true").unwrap_or(false);

    if let Ok(conn) = Connection::open("turnos.db") {
        let query = if ver_todo {
            "SELECT id, fecha, hora, nombre, apellido, dni, telefono, email FROM turnos ORDER BY fecha ASC, hora ASC"
        } else {
            "SELECT id, fecha, hora, nombre, apellido, dni, telefono, email FROM turnos WHERE fecha >= DATE('now', 'localtime') ORDER BY fecha ASC, hora ASC"
        };

        if let Ok(mut stmt) = conn.prepare(query) {
            if let Ok(mut rows) = stmt.query([]) {
                while let Ok(Some(row)) = rows.next() {
                    lista_turnos.push(TurnoDB {
                        id: row.get(0).unwrap_or(0),
                        fecha: row.get(1).unwrap_or_default(),
                        hora: row.get(2).unwrap_or_default(),
                        nombre: row.get(3).unwrap_or_default(),
                        apellido: row.get(4).unwrap_or_default(),
                        dni: row.get(5).unwrap_or_default(),
                        telefono: row.get(6).unwrap_or_default(),
                        email: row.get(7).unwrap_or_default(),
                    });
                }
            }
        }
    }

    if let Ok(tmpl) = env.get_template("admin") {
        match tmpl.render(context! { turnos => lista_turnos, historial => ver_todo }) {
            Ok(html_renderizado) => { res.render(Text::Html(html_renderizado)); }
            Err(e) => {
                println!("Error al renderizar admin.html: {:?}", e);
                res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
    }
}

/// Elimina físicamente una solicitud de turno del motor relacional mediante su identificador único.
#[handler]
pub async fn borrar_turno(req: &mut Request, res: &mut Response) {
    let id = req.params().get("id").map(|s| s.as_str()).unwrap_or_default();

    if id.is_empty() {
        res.status_code(StatusCode::BAD_REQUEST);
        res.render(Text::Plain("Error: ID ausente."));
        return;
    }

    match Connection::open("turnos.db") {
        Ok(conn) => {
            match conn.execute("DELETE FROM turnos WHERE id = ?", [id]) {
                Ok(_) => {
                    println!("Turno ID #{} eliminado desde panel de control.", id);
                    res.render(Text::Plain("OK"));
                }
                Err(e) => {
                    println!("Error SQLite al borrar registro: {:?}", e);
                    res.status_code(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(_) => { res.status_code(StatusCode::INTERNAL_SERVER_ERROR); }
    }
}

// --- UTILIDADES ---

/// Recupera e inyecta en la respuesta de red los activos estáticos del sistema deduciendo su Content-Type (MIME).
#[handler]
pub async fn servir_estaticos(req: &mut Request, res: &mut Response) {
    let mut path = req.params().get("path").map(|s| s.as_str()).unwrap_or_default();

    if let Some(stripped) = path.strip_prefix("static/") {
        path = stripped;
    }

    let path_limpio = path.trim_start_matches('/');

    if let Some(asset) = StaticAssets::get(path_limpio) {
        let mime = mime_guess::from_path(path_limpio).first_or_octet_stream();
        res.headers_mut().insert(header::CONTENT_TYPE, mime.as_ref().parse().unwrap());
        res.write_body(asset.data.into_owned()).unwrap();
    } else {
        println!("404 de Asset Estático: No se encontró '{}' en el binario.", path_limpio);
        res.status_code(StatusCode::NOT_FOUND);
    }
}

// --- CATCH-ALL GLOBAL (404 PERSONALIZADO) ---

/// Intercepta llamadas dirigidas a rutas inexistentes del servidor web y sirve la interfaz visual institucional de error.
#[handler]
pub async fn pagina_no_encontrada(res: &mut Response) {
    let env = crear_entorno_templates();

    if let Ok(tmpl) = env.get_template("404") {
        if let Ok(html_renderizado) = tmpl.render(context! {}) {
            res.status_code(StatusCode::NOT_FOUND);
            res.render(Text::Html(html_renderizado));
            return;
        }
    }

    res.status_code(StatusCode::NOT_FOUND);
    res.render(Text::Plain("Página no encontrada - Control Institucional"));
}