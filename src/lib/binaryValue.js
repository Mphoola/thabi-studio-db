/**
 * Binary cell helpers for the value inspector. A binary cell carries its
 * first bytes as hex (`preview`) and, when small enough, every byte as
 * base64 (`inline`); see `Cell::binary` in Rust.
 */

const IMAGE_SIGNATURES = [
  { hex: "89504e470d0a1a0a", mime: "image/png" },
  { hex: "ffd8ff", mime: "image/jpeg" },
  { hex: "474946383", mime: "image/gif" },
];

/** The image type the bytes start with, or null. WebP is RIFF….WEBP. */
export function imageMime(previewHex) {
  const hex = (previewHex ?? "").toLowerCase();
  const match = IMAGE_SIGNATURES.find((s) => hex.startsWith(s.hex));
  if (match) return match.mime;
  if (hex.startsWith("52494646") && hex.slice(16, 24) === "57454250")
    return "image/webp";
  return null;
}

/** Every byte as hex (0x…), from base64. */
export function base64ToHex(base64) {
  const binary = atob(base64);
  let hex = "0x";
  for (let i = 0; i < binary.length; i += 1)
    hex += binary.charCodeAt(i).toString(16).padStart(2, "0");
  return hex;
}

/**
 * A classic hex dump (offset, 16 bytes, printable characters), capped so a
 * large value doesn't freeze the dialog.
 */
export function hexDump(base64, maxBytes = 4096) {
  const binary = atob(base64);
  const lines = [];
  const end = Math.min(binary.length, maxBytes);
  for (let offset = 0; offset < end; offset += 16) {
    const chunk = binary.slice(offset, Math.min(offset + 16, end));
    const hex = [...chunk]
      .map((c) => c.charCodeAt(0).toString(16).padStart(2, "0"))
      .join(" ")
      .padEnd(47, " ");
    const text = [...chunk]
      .map((c) => {
        const code = c.charCodeAt(0);
        return code >= 32 && code < 127 ? c : ".";
      })
      .join("");
    lines.push(`${offset.toString(16).padStart(8, "0")}  ${hex}  ${text}`);
  }
  if (binary.length > end)
    lines.push(`… ${(binary.length - end).toLocaleString()} more bytes`);
  return lines.join("\n");
}
