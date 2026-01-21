# Datastar Patch Elements & SSE Utilities

## Aktuelle Probleme / Fixes

- **Obsolete Funktionen entfernen**  
  - `remove_empty_lines`  
  - `remove_line_breaks`  

- **HTML Patch Handling verbessern**  
  - Momentan werden nur einzeilige HTML-Elemente korrekt eingefügt.  
  - Beispielproblem:  

    ```text
    event: datastar-patch-elements
    data: elements <div id="header" datastar-patch="replace">
    data:             test
    data:             </div>
    ```

    → `elements` fehlen bei mehrzeiligen Inhalten, weswegen es nicht in DOM gerendert wird.

## Datastar Patch Optionen

| Option | Beschreibung |
|--------|--------------|
| `selector #foo` | Selects the target element of the patch using a CSS selector. Not required when using the outer or replace modes. |
| `mode outer` | Morphs the outer HTML of the elements. Default (and recommended) mode. |
| `mode inner` | Morphs the inner HTML of the elements. |
| `mode replace` | Replaces the outer HTML of the elements. |
| `mode prepend` | Prepends the elements to the target’s children. |
| `mode append` | Appends the elements to the target’s children. |
| `mode before` | Inserts the elements before the target as siblings. |
| `mode after` | Inserts the elements after the target as siblings. |
| `mode remove` | Removes the target elements from DOM. |
| `namespace svg` | Patch elements into the DOM using an SVG namespace. |
| `namespace mathml` | Patch elements into the DOM using a MathML namespace. |
| `useViewTransition true` | Whether to use view transitions when patching elements. Defaults to false. |
| `elements` | The HTML elements to patch. |

## Entwicklungs-/Produktions-Modus

- **Production:**  
  - HTML soll getrimmt werden (Whitespace entfernen).  
- **Development:**  
  - HTML unformatiert lassen oder besser noch über einen Formatter laufen lassen.  

## Ziele / Todos

1. **Utility schreiben**  
   - Vereinfachtes Schreiben von `patch_stream`-Methoden.  
   - Unterstützung für mehrere Stream-Elemente gleichzeitig.  
   - Richtige Handhabung mehrzeiliger HTML-Elemente in `elements`.

2. **Langfristig**  
   - Eventuell HTML-Syntax-Erweiterung über Strings oder Iteratoren ermöglichen.

3. **Showcase / Tests**  
   - `temp_counter.rs` als Demonstration / Showcase verwenden.
