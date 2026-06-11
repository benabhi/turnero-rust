use serde::{Serialize, Deserialize};

/// Estructura de transferencia de datos que representa un bloque de tiempo de atención presencial
/// y su estado de ocupación actual en el sistema.
///
/// El atributo `#[derive(Serialize)]` genera automáticamente la lógica necesaria en tiempo de
/// compilación para transformar esta estructura a formatos de intercambio como JSON o contextos de MiniJinja.
#[derive(Serialize)]
pub struct Horario {
    pub hora: String,
    pub disponible: bool,
    pub mensaje: String,
}

/// Contenedor de mensajes de error específicos para la validación de campos del formulario público.
///
/// # Mecanismo de Rust (Tricky Part):
/// 1. `Option<String>`: En Rust no existe el concepto de puntero nulo (`null`). Para representar la ausencia
///    de un valor de forma segura en tiempo de compilación se utiliza el tipo envoltura `Option<T>`, el cual puede
///    tomar la variante `Some(T)` (contiene un valor) o `None` (está vacío). Esto obliga al desarrollador a
///    evaluar explícitamente ambos casos, eliminando los errores de excepción de puntero nulo en ejecución.
/// 2. `#[derive(Default)]`: Implementa de forma automática el rasgo (trait) `Default`. Esto permite instanciar
///    la estructura inicializando absolutamente todos sus campos de tipo `Option` en la variante `None` mediante
///    la llamada limpia `ErroresFormulario::default()`, actuando como un constructor estándar predecible.
#[derive(Serialize, Default)]
pub struct ErroresFormulario {
    pub fecha: Option<String>,
    pub hora: Option<String>,
    pub nombre: Option<String>,
    pub apellido: Option<String>,
    pub dni: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
}

/// Contenedor de mensajes de error de entrada correspondientes al módulo de autenticación administrativa.
///
/// Al igual que `ErroresFormulario`, utiliza la inicialización por defecto para simplificar el flujo de control
/// cuando las credenciales ingresadas no presentan fallas de formato inicial.
#[derive(Serialize, Default)]
pub struct ErroresLogin {
    pub usuario: Option<String>,
    pub clave: Option<String>,
}

/// Estado y contexto de persistencia temporal requerido por el motor de plantillas MiniJinja
/// para renderizar la interfaz visual del formulario web pública.
///
/// Almacena tanto la información de los bloques horarios calculados para el día solicitado como los
/// valores cargados previamente por el ciudadano (`old_fields`) para evitar que pierda la información ingresada
/// en caso de ocurrir un fallo de validación del lado del servidor.
#[derive(Serialize)]
pub struct ContextoFormulario {
    pub old_fecha: String,
    pub min_fecha: String,
    pub old_hora: String,
    pub old_nombre: String,
    pub old_apellido: String,
    pub old_dni: String,
    pub old_telefono: String,
    pub old_email: String,
    pub horarios: Vec<Horario>,
    pub errores: ErroresFormulario,
    pub es_fin_de_semana: bool,
}

/// Entidad de dominio que agrupa los campos sanitizados del formulario al momento de procesar
/// el alta de una nueva solicitud de asistencia.
///
/// # Mecanismo de Rust (Tricky Part):
/// 1. `#[allow(dead_code)]`: Es un atributo de control que le indica al compilador de Rust que no emita advertencias
///    (*warnings*) si detecta que la estructura o alguno de sus campos no están siendo leídos o instanciados de forma
///    directa en alguna sección aislada del backend. Es útil al desarrollar APIs modulares o sistemas basados en macros.
/// 2. `#[derive(Deserialize)]`: A diferencia de `Serialize`, este rasgo dota a la estructura de la capacidad de
///    tomar una carga de datos estructurada externa (por ejemplo, los datos URL-Encoded enviados por el formulario web)
///    y parsearla de forma automática hacia los tipos fuertemente tipados de Rust correspondientes a cada campo.
#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
pub struct DatosTurno {
    pub fecha: String,
    pub hora: String,
    pub nombre: String,
    pub apellido: String,
    pub dni: String,
    pub telefono: String,
    pub email: String,
}

/// Entidad de persistencia que representa un registro completo de un turno recuperado desde el archivo de base de datos SQL.
///
/// Incluye el identificador correlativo único autoincremental `id` generado de forma nativa por el motor de SQLite,
/// mapeando los tipos primitivos del motor relacional con tipos nativos e inmutables del backend de Rust (`i32` y `String`).
#[derive(Serialize, Debug)]
pub struct TurnoDB {
    pub id: i32,
    pub fecha: String,
    pub hora: String,
    pub nombre: String,
    pub apellido: String,
    pub dni: String,
    pub telefono: String,
    pub email: String,
}