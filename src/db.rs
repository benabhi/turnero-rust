use rusqlite::{Connection, Result as SqlResult};
use std::collections::HashSet;

/// Inicializa el archivo local de la base de datos y crea la estructura de tablas principal.
///
/// Evalúa de forma perezosa la existencia previa de la tabla `turnos` para evitar colisiones
/// o sobreescritura de registros durante los reinicios del servicio.
///
/// # Mecanismo de Rust (Tricky Part):
/// El operador de sufijo `?` es azúcar sintáctico para la propagación de errores acoplada al tipo enum `Result`.
/// Si la función `Connection::open()` o `.execute()` retornan un caso `Ok(T)`, el compilador extrae automáticamente
/// el valor interno y continúa la ejecución en la línea siguiente. Si retornan un caso `Err(E)`, la función actual
/// aborta inmediatamente y propaga de forma automática dicho error hacia el llamador superior (`main.rs`),
/// realizando conversiones de tipo implícitas mediante el rasgo `From`.
///
/// # Parámetros de Consulta:
/// El arreglo vacío `[]` enviado como segundo argumento indica que la sentencia SQL de inicialización
/// no requiere vinculación de variables (*positional parameters*).
pub fn inicializar_db() -> SqlResult<()> {
    // Abre el descriptor del archivo o crea la base de datos embebida si no existe en el disco rígido.
    let conn = Connection::open("turnos.db")?;

    // Ejecuta la DDL estructurando los campos sanitizados del ciudadano.
    // La restricción 'UNIQUE(fecha, hora)' garantiza la consistencia transaccional nativa en la base de datos,
    // actuando como la última línea de defensa contra condiciones de carrera (Race Conditions) de reservas en simultáneo.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS turnos (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            fecha TEXT NOT NULL,
            hora TEXT NOT NULL,
            nombre TEXT NOT NULL,
            apellido TEXT NOT NULL,
            dni TEXT NOT NULL,
            telefono TEXT NOT NULL,
            email TEXT NOT NULL,
            fecha_creacion DATETIME DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(fecha, hora)
        )",
        [],
    )?;

    println!("Base de datos 'turnos.db' verificada e inicializada.");
    Ok(())
}

/// Extrae todos los bloques horarios reservados correspondientes a una fecha específica.
///
/// Retorna una colección indexada optimizada para búsquedas en tiempo constante.
///
/// # Mecanismo de Rust (Tricky Part):
/// 1. `HashSet<String>`: En lugar de usar un vector plano (`Vec<T>`) donde verificar la disponibilidad
///    horaria requeriría una búsqueda lineal de complejidad temporal $O(n)$, se utiliza un mapa hash de
///    búsqueda directa con complejidad promedio $O(1)$.
/// 2. `.unwrap()`: El método `.prepare()` devuelve un `Result`. Al usar `.unwrap()`, le indicamos explícitamente
///    el compilador que asumimos que la sintaxis SQL escrita está libre de errores tipográficos. Si hubiera un error
///    en el string SQL, la aplicación provocaría un pánico (`panic!`) en tiempo de ejecución. En producción, esto
///    es seguro únicamente si la consulta está hardcodeada y testeada estáticamente.
/// 3. `row.get::<_, String>(0)`: El operador de inferencia por guion bajo `_` le delega al compilador de Rust la tarea
///    de deducir el tipo del índice de la columna (en este caso, un entero `usize`), basándose en el contexto del método.
pub fn obtener_horas_reservadas(fecha: &str) -> HashSet<String> {
    let mut horas = HashSet::new();

    // Evaluamos la conexión de forma segura. Si el archivo está bloqueado, devolvemos el mapa vacío de forma defensiva.
    if let Ok(conn) = Connection::open("turnos.db") {

        // Preparamos la consulta SQL en memoria.
        let mut stmt = conn.prepare("SELECT hora FROM turnos WHERE fecha = ?").unwrap();

        // Ejecutamos la consulta vinculando dinámicamente la referencia de la fecha solicitada.
        if let Ok(mut rows) = stmt.query([fecha]) {

            // Ciclo iterativo asíncrononizado sobre el cursor de registros de SQLite.
            // 'while let' ejecuta el bloque de código continuamente mientras el desempaquetado de rows.next()
            // resuelva con éxito una opción con datos: 'Ok(Some(row))'.
            while let Ok(Some(row)) = rows.next() {
                if let Ok(hora) = row.get::<_, String>(0) {
                    // Almacenamos el bloque horario en la colección de exclusión.
                    horas.insert(hora);
                }
            }
        }
    }

    horas
}