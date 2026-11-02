use crate::client::{Function, Trigger};

pub fn functions_text(functions: &[Function]) -> String {
    if functions.is_empty() {
        return "No functions.\n".to_string();
    }
    let rows = functions
        .iter()
        .map(|function| {
            vec![
                function.name.clone(),
                function.language.clone(),
                function.cpu.clone(),
                function.memory.clone(),
                bool_text(function.readonly),
            ]
        })
        .collect::<Vec<_>>();
    table(&["NAME", "LANGUAGE", "CPU", "MEMORY", "READONLY"], &rows)
}

pub fn function_text(function: &Function) -> String {
    format!(
        "name: {}\nlanguage: {}\ncpu: {}\nmemory: {}\nreadonly: {}\nexecutable: {}\n",
        function.name,
        function.language,
        function.cpu,
        function.memory,
        bool_text(function.readonly),
        function.executable
    )
}

pub fn triggers_text(triggers: &[Trigger]) -> String {
    if triggers.is_empty() {
        return "No triggers.\n".to_string();
    }
    let rows = triggers
        .iter()
        .map(|trigger| {
            vec![
                trigger.name.clone(),
                trigger.method.clone(),
                trigger.path.clone(),
                trigger.function.clone(),
                bool_text(trigger.readonly),
            ]
        })
        .collect::<Vec<_>>();
    table(&["NAME", "METHOD", "PATH", "FUNCTION", "READONLY"], &rows)
}

pub fn trigger_text(trigger: &Trigger) -> String {
    format!(
        "name: {}\nmethod: {}\npath: {}\nfunction: {}\nreadonly: {}\n",
        trigger.name,
        trigger.method,
        trigger.path,
        trigger.function,
        bool_text(trigger.readonly)
    )
}

fn bool_text(value: bool) -> String {
    if value {
        "true".to_string()
    } else {
        "false".to_string()
    }
}

fn table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut widths: Vec<usize> = headers.iter().map(|header| header.len()).collect();
    for row in rows {
        for (index, cell) in row.iter().enumerate() {
            widths[index] = widths[index].max(cell.len());
        }
    }

    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(format_row(headers, &widths));
    for row in rows {
        let cells = row.iter().map(String::as_str).collect::<Vec<_>>();
        lines.push(format_row(&cells, &widths));
    }
    lines.push(String::new());
    lines.join("\n")
}

fn format_row(cells: &[&str], widths: &[usize]) -> String {
    cells
        .iter()
        .enumerate()
        .map(|(index, cell)| {
            if index + 1 == cells.len() {
                (*cell).to_string()
            } else {
                format!("{cell:<width$}", width = widths[index])
            }
        })
        .collect::<Vec<_>>()
        .join("  ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_an_empty_function_list() {
        assert_eq!(functions_text(&[]), "No functions.\n");
    }

    #[test]
    fn aligns_function_columns() {
        let text = functions_text(&[Function {
            name: "healthz".to_string(),
            language: "rust".to_string(),
            executable: "/data/healthz.wasm".to_string(),
            cpu: "0.1".to_string(),
            memory: "128".to_string(),
            readonly: true,
        }]);
        assert!(text.starts_with("NAME"));
        assert!(text.contains("healthz"));
        assert!(text.contains("true"));
        assert!(!text.contains("/data/healthz.wasm"));
    }
}
