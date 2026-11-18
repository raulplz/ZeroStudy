"""Genera un `contenido.json` sintético (sin datos reales) para las pruebas e2e."""
import hashlib
import json
import math


def pid(*partes):
    return hashlib.sha1("\x1f".join(partes).encode()).hexdigest()[:12]


def contenido():
    unidades = [
        {"id": "rust", "titulo": "Programación en Rust", "corto": "Rust", "color": "#75c4e8", "carpeta": "Rust", "tipo": "clase",
         "resumen": "Rust/Resumen.md", "notas": 4, "bloques_codigo": 12, "adjuntos": 0,
         "sesiones": [{"n": 1, "nota": "Tema 1 - Ownership", "ruta": "Rust/Tema 1 - Ownership.md", "tema": "Ownership", "bytes": 900, "preguntas": 8},
                      {"n": 2, "nota": "Tema 2 - Traits", "ruta": "Rust/Tema 2 - Traits.md", "tema": "Traits", "bytes": 800, "preguntas": 6}]},
        {"id": "redes", "titulo": "Redes de computadores", "corto": "Redes", "color": "#8fbea0", "carpeta": "Redes", "tipo": "propio",
         "resumen": "Redes/Resumen.md", "notas": 3, "bloques_codigo": 1, "adjuntos": 0,
         "sesiones": [{"n": 1, "nota": "Tema 1 - Capas", "ruta": "Redes/Tema 1 - Capas.md", "tema": "Modelo de capas", "bytes": 700, "preguntas": 6}]},
    ]
    banco = [
        ("rust", "Tema 1 - Ownership", "Préstamos", 1, True, "¿Cuántos propietarios tiene un valor en Rust?", ["Uno", "Dos", "Ninguno", "Tantos como referencias"], 0),
        ("rust", "Tema 1 - Ownership", "Préstamos", 1, True, "¿Qué ocurre al mover un `String`?", ["Se copia", "El origen deja de ser válido", "Se borra", "Nada"], 1),
        ("rust", "Tema 1 - Ownership", "Préstamos", 1, True, "¿Cuántas referencias mutables simultáneas se permiten?", ["Una", "Dos", "Ilimitadas", "Ninguna"], 0),
        ("rust", "Tema 1 - Ownership", "Lifetimes", 1, True, "¿Para qué sirven los lifetimes?", ["Acelerar", "Validar referencias", "Compilar", "Formatear"], 1),
        ("rust", "Tema 1 - Ownership", "Lifetimes", 1, True, "¿`'static` significa?", ["Vive todo el programa", "Es constante", "Es global mutable", "Es estático en C"], 0),
        ("rust", "Tema 2 - Traits", "Traits", 2, True, "¿Qué es un trait?", ["Un tipo", "Un comportamiento compartido", "Un módulo", "Una macro"], 1),
        ("rust", "Tema 2 - Traits", "Traits", 2, True, "¿`dyn Trait` implica?", ["Despacho dinámico", "Despacho estático", "Macro", "Nada"], 0),
        ("rust", "Tema 2 - Traits", "Genéricos", 2, True, "¿Qué hace la monomorfización?", ["Genera código por tipo", "Borra tipos", "Usa vtable", "Interpreta"], 0),
        ("redes", "Tema 1 - Capas", "OSI", 1, True, "¿Qué capa OSI enruta paquetes?", ["Enlace", "Red", "Transporte", "Aplicación"], 1),
        ("redes", "Tema 1 - Capas", "OSI", 1, True, "¿TCP pertenece a la capa de…?", ["Red", "Transporte", "Enlace", "Física"], 1),
        ("redes", "Tema 1 - Capas", "OSI", 1, True, "¿Qué protocolo resuelve nombres?", ["DNS", "ARP", "ICMP", "DHCP"], 0),
        ("redes", "Tema 1 - Capas", "Direcciones", 1, True, "¿Cuántos bits tiene una dirección IPv4?", ["16", "32", "64", "128"], 1),
    ]
    abiertas = [
        ("rust", "Tema 1 - Ownership", "Préstamos", 1, "Explica la diferencia entre `&T` y `&mut T`.", "`&T` es un préstamo compartido de solo lectura; `&mut T` es exclusivo y permite modificar."),
        ("rust", "Tema 1 - Ownership", "Préstamos", 1, "¿Por qué Rust no necesita recolector de basura?", "Porque el ownership libera la memoria al salir de ámbito (RAII)."),
        ("rust", "Tema 2 - Traits", "Traits", 2, "¿Cuándo usar `impl Trait` frente a `dyn Trait`?", "`impl Trait` para despacho estático; `dyn` cuando el tipo se decide en ejecución."),
        ("redes", "Tema 1 - Capas", "OSI", 1, "Describe el saludo de tres vías de TCP.", "SYN, SYN-ACK y ACK establecen la conexión."),
        ("redes", "Tema 1 - Capas", "OSI", 1, "¿Qué diferencia hay entre un switch y un router?", "El switch reenvía por MAC en la misma red; el router enruta por IP entre redes."),
        ("rust", "Tema 2 - Traits", "Genéricos", 2, "¿Qué es un tipo asociado en un trait?", "Un tipo que el implementador fija, como `Item` en `Iterator`."),
    ]
    preguntas = []
    for u, nota, sec, ses, test, enun, ops, ok in banco:
        letras = "abcd"
        preguntas.append({
            "id": pid(u, nota, enun), "unidad": u, "nota": nota, "ruta": f"{ {'rust': 'Rust', 'redes': 'Redes'}[u] }/{nota}.md",
            "seccion": sec, "sesion": ses, "test": test, "pregunta": enun,
            "opciones": "\n".join(f"{letras[i]}) {t}" for i, t in enumerate(ops)),
            "respuesta": f"**Respuesta: {letras[ok]})** {ops[ok]}.",
        })
    for u, nota, sec, ses, enun, resp in abiertas:
        preguntas.append({"id": pid(u, nota, enun), "unidad": u, "nota": nota, "ruta": f"{ {'rust': 'Rust', 'redes': 'Redes'}[u] }/{nota}.md",
                          "seccion": sec, "sesion": ses, "test": False, "pregunta": enun, "opciones": "", "respuesta": resp})
    nodos = [
        {"id": "rust-ownership", "nombre": "Ownership", "definicion": "Cada valor tiene un único dueño.", "nota": "Tema 1 - Ownership", "seccion": "Préstamos", "sesion": 1, "tipo": "concepto", "importancia": 5, "unidad": "rust"},
        {"id": "rust-borrowing", "nombre": "Préstamos", "definicion": "Referencias que no toman la propiedad.", "nota": "Tema 1 - Ownership", "seccion": "Préstamos", "sesion": 1, "tipo": "concepto", "importancia": 4, "unidad": "rust"},
        {"id": "rust-lifetimes", "nombre": "Lifetimes", "definicion": "Anotaciones de validez de referencias.", "nota": "Tema 1 - Ownership", "seccion": "Lifetimes", "sesion": 1, "tipo": "concepto", "importancia": 3, "unidad": "rust"},
        {"id": "rust-traits", "nombre": "Traits", "definicion": "Comportamiento compartido entre tipos.", "nota": "Tema 2 - Traits", "seccion": "Traits", "sesion": 2, "tipo": "concepto", "importancia": 4, "unidad": "rust"},
        {"id": "rust-generics", "nombre": "Genéricos", "definicion": "Código parametrizado por tipo.", "nota": "Tema 2 - Traits", "seccion": "Genéricos", "sesion": 2, "tipo": "concepto", "importancia": 3, "unidad": "rust"},
        {"id": "redes-osi", "nombre": "Modelo OSI", "definicion": "Siete capas de red.", "nota": "Tema 1 - Capas", "seccion": "OSI", "sesion": 1, "tipo": "concepto", "importancia": 5, "unidad": "redes"},
        {"id": "redes-tcp", "nombre": "TCP", "definicion": "Transporte fiable orientado a conexión.", "nota": "Tema 1 - Capas", "seccion": "OSI", "sesion": 1, "tipo": "concepto", "importancia": 4, "unidad": "redes"},
        {"id": "redes-ip", "nombre": "IP", "definicion": "Direccionamiento y enrutado.", "nota": "Tema 1 - Capas", "seccion": "Direcciones", "sesion": 1, "tipo": "concepto", "importancia": 3, "unidad": "redes"},
    ]
    aristas = [
        {"de": "rust-ownership", "a": "rust-borrowing", "tipo": "incluye", "etiqueta": "", "cruzada": False},
        {"de": "rust-borrowing", "a": "rust-lifetimes", "tipo": "requiere", "etiqueta": "", "cruzada": False},
        {"de": "rust-traits", "a": "rust-generics", "tipo": "usa", "etiqueta": "", "cruzada": False},
        {"de": "redes-osi", "a": "redes-tcp", "tipo": "incluye", "etiqueta": "", "cruzada": False},
        {"de": "redes-osi", "a": "redes-ip", "tipo": "incluye", "etiqueta": "", "cruzada": False},
        {"de": "rust-ownership", "a": "redes-tcp", "tipo": "analogia", "etiqueta": "propiedad de un recurso", "cruzada": True},
    ]
    for i, n in enumerate(nodos):  # el lector de la bóveda los trae ya colocados: aquí, en círculo
        a = 2 * math.pi * i / len(nodos)
        n["x"], n["y"] = round(300 * math.cos(a), 1), round(300 * math.sin(a), 1)
    return {
        "boveda": "Apuntes", "generado": "2026-11-17T12:00", "unidades": unidades, "preguntas": preguntas,
        "plazos": [{"fecha": "2026-12-20", "unidad": "rust", "que": "Entrega del proyecto", "estado": "pendiente", "nota": None}],
        "conceptos": {"nodos": nodos, "aristas": aristas},
    }


if __name__ == "__main__":
    print(json.dumps(contenido(), ensure_ascii=False))
