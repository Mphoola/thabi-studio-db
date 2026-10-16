//! Ways back in when the app password is forgotten (see
//! docs/decisions/0033-app-password.md): one-time recovery codes, made when
//! the password is set, and optional security questions.
//!
//! Like the password, only Argon2id hashes are kept, in the OS credential
//! store. A code works once. Either way in ends with a new password, so the
//! app is never left unlocked without one.

use crate::app_lock;
use crate::credentials::CredentialStore;
use crate::errors::AppError;
use argon2::password_hash::rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const CODE_COUNT: usize = 10;
pub const QUESTION_COUNT: usize = 3;
const CODES_REFERENCE: &str = "app-recovery-codes";
const QUESTIONS_REFERENCE: &str = "app-security-questions";

/// Ten codes like `4f1c9-b07ae`: 40 random bits each.
pub fn generate_codes() -> Vec<String> {
    (0..CODE_COUNT)
        .map(|_| {
            let mut bytes = [0u8; 5];
            OsRng.fill_bytes(&mut bytes);
            let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
            format!("{}-{}", &hex[..5], &hex[5..])
        })
        .collect()
}

/// A typed code without case, spaces or the dash.
pub fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// An answer without case or extra spaces, so "  Lilongwe " matches "lilongwe".
pub fn normalize_answer(answer: &str) -> String {
    answer
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn read_json<T: for<'de> Deserialize<'de>>(
    credentials: &dyn CredentialStore,
    reference: &str,
) -> Result<Option<T>, AppError> {
    credentials
        .get(reference)?
        .map(|text| serde_json::from_str(&text).map_err(|_| AppError::storage()))
        .transpose()
}

fn write_json<T: Serialize>(
    credentials: &dyn CredentialStore,
    reference: &str,
    value: &T,
) -> Result<(), AppError> {
    let text = serde_json::to_string(value).map_err(|_| AppError::storage())?;
    credentials.set(reference, &text)
}

/// Replaces the saved codes with hashes of `codes`.
pub fn store_codes(credentials: &dyn CredentialStore, codes: &[String]) -> Result<(), AppError> {
    let hashes = codes
        .iter()
        .map(|code| app_lock::hash(&normalize_code(code)))
        .collect::<Result<Vec<_>, _>>()?;
    write_json(credentials, CODES_REFERENCE, &hashes)
}

/// Uses up a code. True when it was one of the unused ones.
pub fn use_code(credentials: &dyn CredentialStore, code: &str) -> Result<bool, AppError> {
    let code = normalize_code(code);
    let mut hashes: Vec<String> = read_json(credentials, CODES_REFERENCE)?.unwrap_or_default();
    let Some(index) = hashes
        .iter()
        .position(|stored| app_lock::verify(stored, &code))
    else {
        return Ok(false);
    };
    hashes.remove(index);
    write_json(credentials, CODES_REFERENCE, &hashes)?;
    Ok(true)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionInput {
    pub question: String,
    pub answer: String,
}

#[derive(Serialize, Deserialize)]
struct StoredQuestion {
    question: String,
    answer_hash: String,
}

/// Three different questions, each with an answer of at least 3 characters.
pub fn validate_questions(questions: &[QuestionInput]) -> Result<(), AppError> {
    let mut fields = BTreeMap::new();
    if questions.len() != QUESTION_COUNT {
        return Err(AppError::new("validation", "Choose three questions."));
    }
    for (index, input) in questions.iter().enumerate() {
        let question = input.question.trim();
        if !(5..=120).contains(&question.chars().count()) {
            fields.insert(
                format!("question{index}"),
                "Choose or write a question.".into(),
            );
        } else if questions[..index]
            .iter()
            .any(|earlier| earlier.question.trim().eq_ignore_ascii_case(question))
        {
            fields.insert(
                format!("question{index}"),
                "Choose a different question.".into(),
            );
        }
        let answer = normalize_answer(&input.answer);
        if answer.chars().count() < 3 || answer.chars().count() > 120 {
            fields.insert(format!("answer{index}"), "Use 3 to 120 characters.".into());
        }
    }
    if fields.is_empty() {
        Ok(())
    } else {
        Err(AppError::validation(fields))
    }
}

pub fn store_questions(
    credentials: &dyn CredentialStore,
    questions: &[QuestionInput],
) -> Result<(), AppError> {
    let stored = questions
        .iter()
        .map(|input| {
            Ok(StoredQuestion {
                question: input.question.trim().to_owned(),
                answer_hash: app_lock::hash(&normalize_answer(&input.answer))?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    write_json(credentials, QUESTIONS_REFERENCE, &stored)
}

/// True only when every answer matches, in order.
pub fn check_answers(
    credentials: &dyn CredentialStore,
    answers: &[String],
) -> Result<bool, AppError> {
    let stored: Vec<StoredQuestion> =
        read_json(credentials, QUESTIONS_REFERENCE)?.unwrap_or_default();
    Ok(!stored.is_empty()
        && stored.len() == answers.len()
        && stored.iter().zip(answers).all(|(question, answer)| {
            app_lock::verify(&question.answer_hash, &normalize_answer(answer))
        }))
}

pub fn delete_questions(credentials: &dyn CredentialStore) -> Result<(), AppError> {
    credentials.delete(QUESTIONS_REFERENCE)
}

/// Removes every way back in, with the password they belong to.
pub fn delete_all(credentials: &dyn CredentialStore) -> Result<(), AppError> {
    credentials.delete(CODES_REFERENCE)?;
    delete_questions(credentials)
}

/// What the lock screen can offer. Questions are shown while locked, as on
/// any sign-in page; answers and codes never leave the credential store.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryOptions {
    pub codes_left: usize,
    pub questions: Vec<String>,
}

pub fn options(credentials: &dyn CredentialStore) -> Result<RecoveryOptions, AppError> {
    let codes: Vec<String> = read_json(credentials, CODES_REFERENCE)?.unwrap_or_default();
    let questions: Vec<StoredQuestion> =
        read_json(credentials, QUESTIONS_REFERENCE)?.unwrap_or_default();
    Ok(RecoveryOptions {
        codes_left: codes.len(),
        questions: questions.into_iter().map(|q| q.question).collect(),
    })
}

/// The text file offered when codes are made.
pub fn codes_file(codes: &[String]) -> String {
    let mut text = String::from(
        "Thabi Studio recovery codes\n\n\
         Each code can be used once to set a new app password if you forget it.\n\
         Keep them somewhere safe, away from this computer.\n\n",
    );
    for code in codes {
        text.push_str(code);
        text.push('\n');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, sync::Mutex};

    #[derive(Default)]
    struct Store(Mutex<HashMap<String, String>>);

    impl CredentialStore for Store {
        fn set(&self, reference: &str, value: &str) -> Result<(), AppError> {
            self.0
                .lock()
                .unwrap()
                .insert(reference.into(), value.into());
            Ok(())
        }
        fn get(&self, reference: &str) -> Result<Option<String>, AppError> {
            Ok(self.0.lock().unwrap().get(reference).cloned())
        }
        fn delete(&self, reference: &str) -> Result<(), AppError> {
            self.0.lock().unwrap().remove(reference);
            Ok(())
        }
    }

    fn question(question: &str, answer: &str) -> QuestionInput {
        QuestionInput {
            question: question.into(),
            answer: answer.into(),
        }
    }

    #[test]
    fn codes_are_unique_and_formatted() {
        let codes = generate_codes();
        assert_eq!(codes.len(), CODE_COUNT);
        assert!(
            codes
                .iter()
                .all(|code| code.len() == 11 && &code[5..6] == "-")
        );
        let mut unique = codes.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), CODE_COUNT);
    }

    #[test]
    fn each_code_works_once_however_it_is_typed() {
        let store = Store::default();
        let codes = vec!["4f1c9-b07ae".to_owned(), "00000-11111".to_owned()];
        store_codes(&store, &codes).unwrap();
        assert!(use_code(&store, " 4F1C9 B07AE ").unwrap());
        assert!(!use_code(&store, "4f1c9-b07ae").unwrap(), "used up");
        assert!(!use_code(&store, "99999-99999").unwrap());
        assert_eq!(options(&store).unwrap().codes_left, 1);
    }

    #[test]
    fn answers_must_all_match_ignoring_case_and_spaces() {
        let store = Store::default();
        let questions = vec![
            question("What city were you born in?", "Lilongwe"),
            question("What was your first pet's name?", "Tabby"),
            question("What is your favourite book?", "Things Fall Apart"),
        ];
        validate_questions(&questions).unwrap();
        store_questions(&store, &questions).unwrap();
        let answers = |a: &str, b: &str, c: &str| vec![a.to_owned(), b.to_owned(), c.to_owned()];
        assert!(
            check_answers(&store, &answers(" lilongwe", "TABBY", "things  fall apart")).unwrap()
        );
        assert!(
            !check_answers(&store, &answers("Blantyre", "Tabby", "Things Fall Apart")).unwrap()
        );
        assert!(!check_answers(&store, &answers("Lilongwe", "Tabby", "")).unwrap());
        assert_eq!(options(&store).unwrap().questions.len(), 3);
        delete_all(&store).unwrap();
        assert!(
            !check_answers(&store, &answers("Lilongwe", "Tabby", "Things Fall Apart")).unwrap()
        );
    }

    #[test]
    fn questions_must_differ_and_answers_be_long_enough() {
        let error = validate_questions(&[
            question("What city were you born in?", "Lilongwe"),
            question("what city were you born in?", "Zomba"),
            question("What is your favourite book?", "ab"),
        ])
        .unwrap_err();
        let fields = error.fields.unwrap();
        assert!(fields.contains_key("question1"));
        assert!(fields.contains_key("answer2"));
    }
}
