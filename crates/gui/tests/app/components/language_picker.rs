use super::*;

#[test]
fn rows_keep_the_catalogue_order_and_names() {
    let rows = rows_of([("en_us", "English"), ("ru_ru", "Russian")]);
    assert_eq!(
        rows,
        vec![
            Row {
                code: "en_us",
                name: "English",
            },
            Row {
                code: "ru_ru",
                name: "Russian",
            },
        ]
    );
}

#[test]
fn a_chosen_row_reports_its_own_code() {
    let rows = rows_of([("en_us", "English"), ("ru_ru", "Russian")]);
    assert_eq!(code_of(&rows, 0), "en_us");
    assert_eq!(code_of(&rows, 1), "ru_ru");
}

#[test]
fn the_bundled_catalogues_list_english_first() {
    let rows = rows_of(lang::languages().iter().map(|language| (language.code, language.name.as_str())));
    assert_eq!(rows[0].code, lang::DEFAULT);
    assert_eq!(rows[0].name, "English");
}

#[test]
fn the_field_is_the_default_look() {
    assert_eq!(PickerStyle::default(), PickerStyle::Field);
}
