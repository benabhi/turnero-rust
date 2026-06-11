# Sistema Institucional de Reserva de Turnos - Certificado de Antecedentes Penales

Este proyecto consiste en una aplicación web robusta y de alto rendimiento diseñada para la gestión y reserva de turnos presenciales orientados a la tramitación del Certificado de Antecedentes Penales en la Dirección Judicial de la Policía de Río Negro.

Desarrollado como proyecto práctico para la materia **Sistemas de Información 2**, el software implementa una arquitectura backend monolítica, eficiente y autónoma utilizando el lenguaje de programación Rust.

## Descripción del Sistema

El sistema provee una solución digital dividida en dos módulos principales:
1. **Interfaz Pública:** Un formulario web de alta fidelidad que permite a los ciudadanos seleccionar una fecha y un bloque horario disponible (restringido estrictamente a días hábiles futuros y horarios no expirados), sanitizar los datos de identidad (DNI, teléfono, correo) y registrar la solicitud de manera asíncrona mediante peticiones HTTP estructuradas.
2. **Panel Administrativo Interno:** Un módulo protegido mediante sesiones basadas en cookies seguras (`HttpOnly`) que permite al personal oficial auditar las solicitudes en tiempo real a través de una grilla responsiva, filtrar registros históricos y procesar la baja de turnos.

## Arquitectura y Componentes Clave

A diferencia de las arquitecturas web tradicionales basadas en la lectura dinámica de archivos del disco en tiempo de ejecución, este sistema prioriza la seguridad, la velocidad y la portabilidad mediante el paradigma de **activos embebidos**.

* **RustEmbed:** Utilizando macros procedimentales de derivación, todo el directorio de plantillas HTML (`templates/`) y los recursos estáticos como hojas de estilo (`static/estilos.css`) son procesados e inyectados directamente dentro del archivo ejecutable binario final durante la fase de compilación como arreglos estáticos de bytes (`&'static [u8]`).
* **Beneficios de Portabilidad:** El servidor resultante es un único archivo binario autónomo. No requiere de servidores web externos (como Apache o Nginx), intérpretes, ni de la presencia de carpetas adjuntas en el sistema de archivos del entorno de despliegue.
* **Motor de Plantillas MiniJinja:** Implementa un entorno de renderizado de vistas seguro y en memoria que procesa la herencia de layouts institucionales sin realizar llamadas de E/S (*I/O*) bloqueantes al almacenamiento.

## Estructura del Proyecto

El código fuente se organiza bajo los estándares idiomáticos de un *crate* binario de Rust:

```text
├── Cargo.toml                  # Archivo de configuración del proyecto y dependencias (Salvo, Serde, Rusqlite, MiniJinja)
├── static/                     # Recursos estáticos del frontend
│   └── estilos.css             # Hoja de estilos unificada y adaptativa (Responsiva)
├── templates/                  # Vistas del sistema (Compiladas en el binario mediante RustEmbed)
│   ├── 404.html                # Interfaz de captura de rutas inexistentes
│   ├── admin.html              # Panel de control y auditoría de turnos
│   ├── formulario.html         # Formulario público de solicitud de turnos
│   ├── layout.html             # Estructura y navegación común del sistema
│   ├── login.html              # Interfaz de autenticación administrativa
│   └── respuesta.html          # Ticket de confirmación de reserva exitosa
└── src/                        # Código fuente del backend en Rust
    ├── db.rs                   # Inicialización de SQLite y consultas transaccionales de persistencia
    ├── handlers.rs             # Controladores de lógica de negocio, sanitización y enrutamiento web
    ├── main.rs                 # Punto de entrada de la aplicación, configuración de red y árbol de rutas
    └── models.rs               # Estructuras de datos fuertemente tipadas y esquemas de serialización

```

## Requisitos Previos

Para compilar y ejecutar la aplicación desde el código fuente es necesario contar con el conjunto de herramientas de Rust instalado en el sistema:

* Rustc y Cargo (Edición 2021 o superior)
* Un compilador de C compatible (requerido para compilar las extensiones nativas de la biblioteca `rusqlite` encargada de interactuar con el motor SQLite local).

## Instrucciones de Compilación y Optimización

Para compilar el proyecto garantizando el máximo rendimiento de ejecución y la mínima huella de almacenamiento (máxima compresión del ejecutable binario), se deben seguir los siguientes pasos orientados a entornos de producción:

### 1. Configuración de Perfil de Lanzamiento (`Cargo.toml`)

Asegúrese de que el archivo `Cargo.toml` incluya las siguientes directivas de optimización bajo el perfil de liberación (*release profile*) para forzar la reducción del tamaño del binario:

```toml
[profile.release]
opt-level = "z"     # Optimiza el binario priorizando la reducción estricta de tamaño
lto = true          # Habilita la optimización en tiempo de enlace (Link-Time Optimization) entre crates
codegen-units = 1   # Reduce las unidades de generación de código a 1 para maximizar las optimizaciones globales
panic = "abort"     # Elimina la infraestructura de desempaquetado de pánicos, reduciendo tamaño sobrante
strip = true        # Remueve automáticamente todos los símbolos de depuración y tablas de nombres

```

### 2. Comando de Compilación

Ejecute el proceso de compilación comercial utilizando el modificador de optimización de Cargo en la terminal:

```bash
cargo build --release

```

El compilador generará el archivo ejecutable optimizado en la ruta objetivo:
`target/release/turnero` (o `turnero.exe` en entornos Windows).

## Puesta en Marcha (Despliegue)

Siga estos pasos secuenciales para ejecutar la aplicación tanto en entornos de desarrollo local como en servidores de producción:

### Paso 1: Preparación del Entorno

Clone o copie la estructura del proyecto en su directorio de trabajo y asegúrese de situarse en la raíz del mismo (donde se encuentra el archivo `Cargo.toml`):

```bash
cd /ruta/al/proyecto/turnero

```

### Paso 2: Ejecución Directa en Desarrollo

Si desea probar el sistema rápidamente sin generar un binario de producción, utilice el comando de ejecución directa. Esto compilará las dependencias en modo de depuración (*debug*) e iniciará el servicio de inmediato:

```bash
cargo run

```

### Paso 3: Lanzamiento en Producción (Binario Autónomo)

Una vez compilado el binario optimizado mediante las instrucciones de la sección anterior (`cargo build --release`), traslade únicamente el archivo ejecutable (`turnero` o `turnero.exe`) a la carpeta definitiva de despliegue.

Para iniciar el servidor institucional de forma aislada, ejecute el binario desde la terminal de comandos:

```bash
./turnero

```

### Paso 4: Inicialización del Motor de Datos

Al arrancar, el backend comprobará de forma automática la presencia del motor SQLite. Si el archivo no existe, creará de manera autónoma el archivo `turnos.db` en el mismo directorio de ejecución, aplicando el esquema de tablas y restricciones relacionales necesarias para la operación.

### Paso 5: Acceso al Sistema e Interfaces

Una vez que la terminal indique que el servicio está activo, puede abrir cualquier navegador web compatible e ingresar a los siguientes módulos:

* **Módulo Público (Turnero Ciudadano):** `http://localhost:3000/`
* **Módulo de Gestión (Acceso Restringido):** `http://localhost:3000/admin`

*Nota sobre credenciales administrativas:* El acceso por defecto configurado para el personal oficial requiere el usuario `admin` y la contraseña de seguridad `123456`.