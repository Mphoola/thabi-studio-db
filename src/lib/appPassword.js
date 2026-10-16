/**
 * Rules for the app password. Rust enforces the same ones
 * (`app_lock::strength_error`); these drive the live checklist.
 */

export const MIN_PASSWORD_LENGTH = 10;

/** @returns {{ id: string, label: string, met: boolean }[]} */
export function passwordRules(password) {
  const value = password ?? "";
  return [
    {
      id: "length",
      label: `At least ${MIN_PASSWORD_LENGTH} characters`,
      met: [...value].length >= MIN_PASSWORD_LENGTH,
    },
    {
      id: "upper",
      label: "An uppercase letter",
      met: value !== value.toLowerCase(),
    },
    {
      id: "lower",
      label: "A lowercase letter",
      met: value !== value.toUpperCase(),
    },
  ];
}

export function isStrongPassword(password) {
  return passwordRules(password).every((rule) => rule.met);
}

/** Auto-lock choices, in minutes; 0 never locks while the app is open. */
export const AUTO_LOCK_OPTIONS = [
  { value: "1", label: "After 1 minute" },
  { value: "5", label: "After 5 minutes" },
  { value: "15", label: "After 15 minutes" },
  { value: "30", label: "After 30 minutes" },
  { value: "60", label: "After 1 hour" },
  { value: "0", label: "Only at launch" },
];

/** Suggested security questions; people can also write their own. */
export const SECURITY_QUESTIONS = [
  "What city were you born in?",
  "What was the name of your first pet?",
  "What was the name of your primary school?",
  "What is your mother's maiden name?",
  "What was the make of your first car?",
  "What is the name of the street you grew up on?",
  "What was your childhood nickname?",
  "What is your favourite book?",
];

/** How many questions are set, and how many must all be answered. */
export const QUESTION_COUNT = 3;
