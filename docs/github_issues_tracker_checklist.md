# QymCAD — Master Issues & Pull Requests Tracking Checklist

Цей чек-лист складено за результатами аналізу актуальних трекерів та інженерних тем у репозиторії **QymIs-Tech/QymCAD**. Усі перелічені нижче issues наразі перебувають у відкритому стані (Open), і за ними не зафіксовано призначених виконавців (unassigned) чи активних гілок для виправлення у мейнстрімі.

---

## 1. Зведений чек-лист відкритих Issues

| № Issue | Заголовок / Суть проблеми | Категорія | Статус / Призначено |
|---|---|---|---|
| **#153** | Formula case sensitivity: parameter formulas referencing names in another case (`W*2` for `w`) stop following changes. | Bug / Core | 🟢 Ніхто не брав |
| **#152** | Infinite or extreme coordinates (`> 1e15`) in imported drawings cause panics in the weld grid during debug builds. | Bug / IO | 🟢 Ніхто не брав |
| **#151** | Sketch timing race condition: sketches with tight solve limits are saved half-solved depending on execution timing. | Bug / Sketcher | 🟢 Ніхто не брав |
| **#147** | Non-deterministic project serialization: saving the same edit twice yields different `.qcad` files due to HashSet iteration order. | Bug / IO | 🟢 Ніхто не брав |
| **#143** | Self-overlapping parameter renaming: renaming a parameter whose name overlaps itself in a formula triggers a byte range panic. | Bug / Expressions | 🟢 Ніхто не брав |
| **#141** | DXF backwards knot splines: negative or backward knot progression causes clamp panics on the UI thread during import. | Bug / Import | 🟢 Ніхто не брав |
| **#139** | Deeply nested SVG files (thousands of levels) trigger a stack overflow during recursive import parsing. | Bug / Import | 🟢 Ніхто не брав |
| **#136** | Pattern quadratic complexity: linear or circular pattern execution time grows quadratically (`carry_ids` name indexing per copy). | Performance | 🟢 Ніхто не брав |
| **#134** | Deeply nested 3MF / AMF files (tens of thousands of levels) trigger stack overflows during import parsing. | Bug / Import | 🟢 Ніхто не брав |
| **#129** | Feature request: store preview thumbnail inside `.qcad` project bundles. | Enhancement | 🟢 Ніхто не брав |
| **#127** | Failure to render created linear pattern of components (original displays correctly, duplicates fail). | Bug / Assembly | 🟢 Ніхто не брав |
| **#123** | Un-sampleable OCCT body edges: project files containing bodies with edges OCCT cannot sample crash when rendered. | Bug / Kernel | 🟢 Ніхто ne брав |
| **#119** | Rebuild thread panic cascade: when the background rebuild thread panics, all bodies in the document lose live B-rep. | Bug / Core | 🟢 Ніхто не брав |
| **#118** | NaN coordinate meshes: mesh files containing "NaN" coordinates are imported silently and crash Recognise (`index out of bounds`). | Bug / Meshfit | 🟢 Ніхто не брав |
| **#115** | Feature request / Packaging: Sign macOS application bundle. | Packaging | 🟢 Ніхто не брав |
| **#113** | Thread safety race condition: split body pieces are rebuilt on two threads simultaneously sharing a cut face, causing crashes. | Bug / Part | 🟢 Ніхто не брав |

---

## 2. Пріоритетні напрями для виправлення (Triage & Action Plan)

1. **Критична стабільність (Пріоритет P0):**
   - Виправлення падінь потоків перебудови (`#119`) та усунення паралельних гонок потоків під час розбиття тіл (`#113`).
   - Захист від переповнення стеку при імпорті глибоко вкладених форматів SVG / 3MF / AMF (`#139`, `#134`).
2. **Детермінізм даних та точність (Пріоритет P1):**
   - Усунення непередбачуваного порядку збереження в серіалізації (`#147`) та виправлення регістрозалежності формул параметрів (`#153`).
   - Валідація екстремальних координат (`#152`, `#118`) задля запобігання падінням під час виділення сіток.
3. **Нові можливості та покращення (Пріоритет P2):**
   - Збереження прев'ю-мініатюр у проєкті `.qcad` (`#129`).
   - Підписання macOS бінарників (`#115`).
