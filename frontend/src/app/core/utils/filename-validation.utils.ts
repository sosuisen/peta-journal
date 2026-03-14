const FORBIDDEN_CHARS = /[<>:"\\|?*\x00-\x1F\x7F]/;
const RESERVED_NAMES = /^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$/i;

/**
 * Validates a page title for use as a filename on Windows, macOS, and Linux.
 * Returns an error message string if invalid, or null if valid.
 */
export function validateFileName(title: string): string | null {
  const match = FORBIDDEN_CHARS.exec(title);
  if (match) {
    const ch = match[0];
    if (ch.charCodeAt(0) < 0x20 || ch.charCodeAt(0) === 0x7F) {
      return 'Title contains a control character (not allowed in filenames)';
    }
    return `Title contains '${ch}' (not allowed in filenames)`;
  }
  if (title.startsWith('/') || title.endsWith('/')) {
    return "Title cannot start or end with '/'";
  }
  if (title.endsWith('.') || title.endsWith(' ')) {
    return 'Title cannot end with a dot or space';
  }
  if (RESERVED_NAMES.test(title)) {
    return `Title '${title}' is a reserved name on Windows`;
  }
  return null;
}
