# Довідник команд терміналу QymCAD

Цей документ містить повний перелік команд терміналу для компіляції, запуску, тестування, перевірки якості коду та пакування проєкту **QymCAD**.

---

## 1. Базові передумови та оточення

Перед початком переконайтеся, що встановлено актуальний Rust toolchain:
```bash
# Перевірка версії rustc та cargo
rustc --version
cargo --version

# Оновлення Rust до актуальної стабільної версії
rustup update stable

# Додавання компонентів форматування та лінтера
rustup component add rustfmt clippy
```

---

## 2. Компіляція та перевірка коду

### Швидка перевірка синтаксису й типів (без повної збірки)
Найшвидший спосіб дізнатися про помилки компіляції під час написання коду:
```bash
# Перевірка всього робочого простору (workspace)
cargo check --workspace --all-targets

# Перевірка конкретного крейту
cargo check -p qymcad
cargo check -p qymcad-core
cargo check -p qymcad-part
```

### Компіляція (Debug-профіль)
```bash
# Збірка головного застосунку
cargo build --bin qymcad

# Збірка всіх бібліотек та бінарників робочого простору
cargo build --workspace
```

### Оптимізована компіляція (Release-профіль)
```bash
# Збірка релізної версії для максимальної швидкодії
cargo build --release --bin qymcad
```

<!--
### Допоміжний бінарник AI / MCP Server
```bash
# Збірка сервера Model Context Protocol (QymCAD AI Copilot)
cargo build --bin qymcad_mcp
```
-->

---

## 3. Запуск застосунку

### Звичайний запуск (Debug)
```bash
# Запуск головного графічного інтерфейсу QymCAD
cargo run --bin qymcad

# Запуск через Justfile (якщо встановлено утиліту just)
just dev
```

### Запуск із відкриттям файлу проєкту
```bash
# Передача шляху до файлу моделі (.qcad)
cargo run --bin qymcad -- /шлях/до/моделі.qcad
```

### Запуск релізної версії
```bash
cargo run --release --bin qymcad
```

<!--
### Запуск MCP-сервера для штучного інтелекту
```bash
cargo run --bin qymcad_mcp
```
-->

---

## 4. Тестування

### Запуск усіх тестів
```bash
# Усі тести всіх крейтів у workspace
cargo test --workspace

# Тести через Justfile
just test
```

### Тести окремих крейтів
```bash
# Тести головного інтерфейсу та застосунку
cargo test -p qymcad

# Тести геометричного ядра та регенерації
cargo test -p qymcad-core

# Тести операцій над деталями (Part Workbench)
cargo test -p qymcad-part

# Тести складальних одиниць (Assembly Workbench)
cargo test -p qymcad-assembly

# Тести ескізника (Sketcher)
cargo test -p qymcad-sketch

# Тести маніпуляторів та стану інтерфейсу
cargo test -p qymcad-ui-state

# Приймальні (acceptance) тести
cargo test -p qymcad-acceptance
```

### Запуск конкретного тесту або модуля
```bash
# Запуск одного конкретного тесту за назвою (наприклад, маніпулятори отвору)
cargo test -p qymcad edit_existing_hole_moves_offsets_and_point

# Запуск групи тестів модуля
cargo test -p qymcad gui::edge_radius_handle_flow

# Запуск тесту з відображенням виводу println! / debug logs
cargo test -p qymcad <назва_тесту> -- --nocapture
```

### Керування потоками тестування
```bash
# Послідовний запуск тестів (в один потік)
cargo test -p qymcad -- --test-threads=1

# Встановлення кількості паралельних потоків через змінну середовища
QYMCAD_TEST_THREADS=4 cargo test --workspace
```

---

## 5. Форматування та лінтери (Контроль якості)

Правила репозиторію вимагають суворого дотримання форматування та відсутності попереджень Clippy (`clippy::all = "deny"`).

### Перевірка та виправлення форматування (rustfmt)
```bash
# Автоматичне форматування всього робочого простору
cargo fmt --all

# Перевірка форматування без внесення змін (використовується в CI)
cargo fmt --all -- --check
```

### Перевірка лінтером Clippy
```bash
# Сувора перевірка всього робочого простору та тестів
cargo clippy --workspace --all-targets
```

### Спеціальні тести-ратчети (Invariants & Guards)
Проєкт використовує тести для запобігання розростанню монолітного об'єкта `App` та збереженню архітектури:
```bash
# Перевірка розміру і методів App
cargo test -p qymcad god_object_ratchet

# Перевірка типів аргументів та сигнатур
cargo test -p qymcad wide_signature_ratchet

# Перевірка відповідності кольорових схем (відсутність захардкодних кольорів)
cargo test -p qymcad no_colour_is_written_as_a_number_in_the_ui
```

---

## 6. Внутрішні скрипти та автоматизація (tools/)

### Ворота якості (Gatekeeper - tools/gate.py)
Скрипт запускає комплексні рівні перевірок перед відправкою змін:
```bash
# Швидкі перевірки (правила коду, швидкі проби)
python3 tools/gate.py fast

# Повний набір перевірок перед створенням коміту
python3 tools/gate.py full

# Релізний набір перевірок (включаючи довгі ланцюжки та тести надійності)
python3 tools/gate.py release

# Запуск конкретного кроку воріт
python3 tools/gate.py full --step <НАЗВА_КРОКУ>
```

### Перевірка локалізації (i18n)
```bash
# Перевірка покриття всіх ключів перекладу
python3 tools/i18n.py

# Перевірка конкретної мови (наприклад, української)
python3 tools/i18n.py uk
just i18n uk
```

### Конвертер файлів старих версій
```bash
# Конвертація файлу моделі в актуальний формат (зі створенням .bak резервної копії)
python3 convert_qcad.py шлях/до/моделі.qcad
```

---

## 7. Збірка релізних пакетів (Packaging)

### Очищення тимчасових файлів збірки
```bash
# Видалення цільової папки target/
cargo clean

# Видалення директорії згенерованих дистрибутивів dist/
just clean-dist
```

### Linux (AppImage через Docker)
Для збірки потрібен встановлений Docker. Ядро OpenCASCADE та glibc компілюються всередині контейнера:
```bash
# Збірка AppImage у папку dist/
just pkg-linux
```

### macOS (Створення .app бандла)
```bash
# Повна збірка релізу та створення бандла через Justfile
just pkg-macos

# Або прямий запуск скрипта (автоматично знаходить OCCT у Homebrew або бере з $OCCT_ROOT):
cargo build --release --bin qymcad
./packaging/macos/bundle.sh
```

---

## 8. Рекомендований робочий процес розробника

Швидкий контрольний ланцюжок команд перед оформленням коміту або створенням Pull Request:

```bash
# 1. Автоформатування
cargo fmt --all

# 2. Перевірка лінтером
cargo clippy --workspace --all-targets

# 3. Запуск релевантних юніт-тестів
cargo test -p qymcad
```
