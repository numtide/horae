//! Test-only readers for the single-sheet workbooks the exports write.

use std::io::{Cursor, Read};

pub(crate) fn xlsx_part(bytes: &[u8], path: &str) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut content = String::new();
    zip.by_name(path)
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();
    content
}

pub(crate) fn assert_cell(sheet: &str, cell: &str, value: &str) {
    let (_, after) = sheet.split_once(&format!("r=\"{cell}\"")).unwrap();
    let (element, _) = after.split_once("</c>").unwrap();
    assert!(
        element.contains(&format!("<v>{value}</v>")),
        "{cell}: {element}"
    );
}

pub(crate) fn assert_text_cell(workbook: &[u8], cell: &str, expected: &str) {
    let sheet = xlsx_part(workbook, "xl/worksheets/sheet1.xml");
    let (_, after) = sheet.split_once(&format!("r=\"{cell}\"")).unwrap();
    let (element, _) = after.split_once("</c>").unwrap();
    assert!(element.contains("t=\"s\""), "{cell}: {element}");
    let index: usize = element
        .split_once("<v>")
        .unwrap()
        .1
        .split_once("</v>")
        .unwrap()
        .0
        .parse()
        .unwrap();
    let strings = xlsx_part(workbook, "xl/sharedStrings.xml");
    let value = strings
        .split("<si>")
        .nth(index + 1)
        .unwrap()
        .split_once("</si>")
        .unwrap()
        .0;
    assert_eq!(value, format!("<t>{expected}</t>"), "{cell}");
}
