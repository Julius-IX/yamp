//! The CSV text format itself. There is no dependency for it, since what is needed is small
//! and the files come from tools that disagree on details like the separator.

/// Splits the text into records of fields. Quoted fields can hold the separator, quotes
/// (doubled) and line breaks. Blank lines are dropped.
pub(crate) fn records(text: &str) -> Vec<Vec<String>> {
  let text = text.strip_prefix('\u{feff}').unwrap_or(text);
  let sep = separator(text);
  let mut rows = Vec::new();
  let mut row = Vec::new();
  let mut field = String::new();
  let mut quoted = false;
  let mut chars = text.chars().peekable();

  let mut end_row = |row: &mut Vec<String>, field: &mut String| {
    row.push(std::mem::take(field));
    let blank = row.len() == 1 && row[0].is_empty();
    let done = std::mem::take(row);
    if !blank {
      rows.push(done);
    }
  };

  while let Some(c) = chars.next() {
    if quoted {
      if c != '"' {
        field.push(c);
      } else if chars.peek() == Some(&'"') {
        field.push('"');
        chars.next();
      } else {
        quoted = false;
      }
    } else if c == '"' && field.is_empty() {
      quoted = true;
    } else if c == sep {
      row.push(std::mem::take(&mut field));
    } else if c == '\n' || c == '\r' {
      if c == '\r' && chars.peek() == Some(&'\n') {
        chars.next();
      }
      end_row(&mut row, &mut field);
    } else {
      field.push(c);
    }
  }
  if !field.is_empty() || !row.is_empty() {
    end_row(&mut row, &mut field);
  }
  rows
}

/// Spreadsheets in some regions save with a semicolon, and some tools use tabs. The header
/// line decides, since a field further down can hold any of them.
fn separator(text: &str) -> char {
  let mut counts = [(',', 0), (';', 0), ('\t', 0)];
  let mut quoted = false;
  for c in text.chars() {
    match c {
      '"' => quoted = !quoted,
      '\n' | '\r' if !quoted => break,
      _ if !quoted => {
        if let Some(entry) = counts.iter_mut().find(|(s, _)| *s == c) {
          entry.1 += 1;
        }
      }
      _ => {}
    }
  }
  // the comma wins a tie, being the usual one
  counts.iter().fold((',', 0), |best, &(s, n)| if n > best.1 { (s, n) } else { best }).0
}

#[cfg(test)]
mod tests {
  use super::*;

  fn view(rows: &[Vec<String>]) -> Vec<Vec<&str>> {
    rows.iter().map(|r| r.iter().map(String::as_str).collect()).collect()
  }

  #[test]
  fn splits_plain_records() {
    let rows = records("a,b\r\n1,2\n3,4");
    assert_eq!(view(&rows), [vec!["a", "b"], vec!["1", "2"], vec!["3", "4"]]);
  }

  #[test]
  fn quoted_fields_keep_separators_quotes_and_breaks() {
    let rows = records("a,b\n\"x, y\",\"say \"\"hi\"\"\"\n\"two\nlines\",z");
    assert_eq!(view(&rows), [vec!["a", "b"], vec!["x, y", "say \"hi\""], vec!["two\nlines", "z"]]);
  }

  #[test]
  fn drops_the_byte_order_mark_and_blank_lines() {
    let rows = records("\u{feff}a,b\n\n1,2\n");
    assert_eq!(view(&rows), [vec!["a", "b"], vec!["1", "2"]]);
  }

  #[test]
  fn keeps_empty_fields() {
    let rows = records("a,b,c\n,,\n1,,3");
    assert_eq!(view(&rows), [vec!["a", "b", "c"], vec!["", "", ""], vec!["1", "", "3"]]);
  }

  #[test]
  fn finds_the_separator_in_the_header() {
    let rows = records("a;b\n1;2,5");
    assert_eq!(view(&rows), [vec!["a", "b"], vec!["1", "2,5"]]);
    let rows = records("a\tb\n1\t2");
    assert_eq!(view(&rows), [vec!["a", "b"], vec!["1", "2"]]);
  }
}
