use salvo::prelude::*;
use salvo::logging::Logger;
use rust_embed::RustEmbed;

// Declaración de la estructura modular del proyecto.
// Nota de Rust: Los módulos se declaran con 'mod' en la raíz (main.rs o lib.rs)
// para que el compilador los incluya en el árbol de compilación del crate.
mod models;
mod db;
mod handlers;

/// Estructura que mapea y embebe el directorio de plantillas HTML (`templates/`)
/// directamente dentro del archivo ejecutable binario final durante la fase de compilación.
///
/// # Mecanismo de Rust (Tricky Part):
/// El atributo `#[derive(RustEmbed)]` utiliza macros de derivación procedimentales. En lugar de
/// leer los archivos desde el disco rígido en tiempo de ejecución (lo que rompería la portabilidad),
/// lee los contenidos del directorio especificado en tiempo de compilación y los inyecta como
/// arreglos estáticos de bytes (`&'static [u8]`) dentro de la estructura.
#[derive(RustEmbed)]
#[folder = "templates/"]
pub struct Templates;

/// Estructura encargada de embeber los activos estáticos del sistema (archivos CSS, JS, imágenes).
///
/// Al igual que `Templates`, garantiza que el ejecutable sea 100% portable y autónomo, eliminando
/// dependencias del entorno de archivos local donde se despliegue el servidor.
#[derive(RustEmbed)]
#[folder = "static/"]
pub struct StaticAssets;

/// Punto de entrada principal de la aplicación.
///
/// # Mecanismo de Rust (Tricky Part):
/// Por defecto, la función `main` en Rust es síncrona y se ejecuta sobre un único hilo nativo.
/// El atributo `#[tokio::main]` es una macro de inicialización que reescribe la firma de la función.
/// Transforma el punto de entrada para que configure e inicie el entorno de ejecución asíncrono de Tokio
/// (un grupo de hilos de trabajo o *thread-pool* administrados mediante un algoritmo de *work-stealing*),
/// permitiendo el uso de la sintaxis `.await` y funciones no bloqueantes en todo el ciclo de vida del software.
#[tokio::main]
async fn main() {
    // Configuración del suscriptor de trazas del sistema (Logging).
    // Se deshabilita el formato ANSI para evitar la inyección de caracteres de escape de color ocultos,
    // garantizando la legibilidad de los logs en consolas secundarias o entornos heredados como Windows CMD.
    tracing_subscriber::fmt()
        .with_env_filter("turnero=debug,salvo=debug")
        .with_ansi(false)
        .init();

    // Inicialización del motor de persistencia de datos SQLite.
    // Mecanismo de Rust (Tricky Part): El operador 'if let' es una estructura de control condicional
    // idiomática que realiza un desempaquetado parcial (*pattern matching*) sobre tipos enum complejos.
    // El método 'db::inicializar_db()' retorna un tipo 'Result<(), Error>'. Al evaluar 'Err(e)',
    // interceptamos exclusivamente el flujo de falla capturando la variable de error 'e' sin necesidad
    // de procesar explícitamente el caso de éxito 'Ok(())'.
    if let Err(e) = db::inicializar_db() {
        eprintln!("Error critico SQLite: {:?}", e);
        return;
    }

    // Configuración del árbol de enrutamiento estructural del framework Salvo.
    // Se utiliza un patrón de diseño de interfaz fluida (Method Chaining) donde cada llamada
    // registra un endpoint o comportamiento y devuelve el objeto Router modificado.
    let router = Router::new()
        // Registra el Middleware de Logging de Salvo. El método '.hoop()' actúa como un gancho o
        // interceptor que procesará todas las peticiones entrantes antes de derivarlas al handler.
        .hoop(Logger::new())

        // Captura y procesa las peticiones de lectura (GET) sobre la raíz del dominio.
        .get(handlers::vista_formulario)

        // Endpoint de procesamiento de solicitudes de reserva de turnos (Envío del Formulario).
        .push(Router::with_path("sacar-turno").post(handlers::procesar_turno))

        // --- MÓDULO ADMINISTRATIVO INSTITUCIONAL ---

        // Enrutador intermedio: intercepta la llamada base y evalúa las cookies de sesión
        // para desviar el tráfico al panel de control o al formulario de login.
        .push(Router::with_path("admin").get(handlers::redireccion_admin))

        // Autenticación: Endpoint dual que sirve la interfaz visual (GET) y procesa
        // las credenciales de acceso institucional (POST).
        .push(
            Router::with_path("admin/login")
                .get(handlers::vista_login)
                .post(handlers::procesar_login)
        )

        // Cierre de Sesión: Endpoint que invalida y purga la cookie de seguridad del cliente.
        .push(Router::with_path("admin/logout").get(handlers::procesar_logout))

        // Monitor de Auditoría: Renderiza la grilla de turnos históricos y vigentes del sistema.
        .push(Router::with_path("admin/turnos").get(handlers::vista_admin))

        // Operaciones de Administración: Endpoint dinámico que recibe el parámetro identificador
        // '<id>' por URL y ejecuta la baja lógica/física del registro en la base de datos.
        .push(Router::with_path("admin/borrar/<id>").post(handlers::borrar_turno))

        // --- RECURSOS ESTÁTICOS Y COMPLEMENTOS ---
        // Manejo de Comodines: El patrón '<*path>' es un selector catch-all que captura
        // de forma recursiva cualquier subruta requerida por los activos del frontend.
        .push(Router::with_path("static/<*path>").get(handlers::servir_estaticos))

        // --- CATCH-ALL GLOBAL (Manejo Controlado de Errores 404) ---
        // Cualquier solicitud HTTP dirigida a una URL inexistente en el árbol estructural
        // es capturada aquí, devolviendo una interfaz visual homogenizada.
        .push(
            Router::with_path("<*path>")
                .get(handlers::pagina_no_encontrada)
                .post(handlers::pagina_no_encontrada)
        );

    // Inicialización de la infraestructura de red.
    // Al bindeamos a la dirección IP comodín "0.0.0.0", el socket de red del sistema operativo queda configurado
    // para escuchar e interactuar con peticiones provenientes tanto de la interfaz loopback local (127.0.0.1)
    // como de adaptadores de red físicos conectados a la LAN corporativa.
    println!("Servidor 'Turnero' corriendo en http://127.0.0.1:3000");
    let acceptor = TcpListener::new("0.0.0.0:3000").bind().await;
    Server::new(acceptor).serve(router).await;
}