// Auto Template: puts the items of a template into a new page, or into a page that gets a tag.
//
// It runs on the journal server (see ADR 009). The templates are the .md files of this
// folder. Each has sections of a frontmatter and a body:
//
//   ---
//   pattern: Monday, weekly
//   tag: book
//   ---
//   The body, in Flat Markdown
//
// The items of a template go to the end of the page. A new page has only one empty item,
// which the first item overwrites. The template of a tag goes in only after the user agrees.

const DAYS = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];

globalThis.onPageCreate = (page) => {
  const matched = loadTemplates().filter((t) => t.patterns.some((p) => matchesPattern(p, page)));
  insertTemplates(matched);
};

globalThis.onPageTagAdd = (page, tags) => {
  const templates = loadTemplates();
  for (const tag of tags) {
    const matched = templates.filter((t) => t.tags.includes(tag));
    if (matched.length === 0) continue;
    if (peta.confirm(`Insert the template of the tag "#${tag}" at the end of this page?`)) {
      insertTemplates(matched);
    }
  }
};

/** The templates of every .md file of this folder, in file name order. */
function loadTemplates() {
  const templates = [];
  for (const name of peta.files()) {
    if (name.toLowerCase().endsWith('.md')) {
      templates.push(...parseTemplates(peta.readFile(name)));
    }
  }
  return templates;
}

function parseTemplates(text) {
  const results = [];
  // The --- lines split the text into a frontmatter and a body, again and again. The text
  // before the first --- is empty.
  const sections = text.split(/^---\r?$/m);
  for (let i = 1; i < sections.length; i += 2) {
    const frontmatter = sections[i].trim();
    const body = i + 1 < sections.length ? sections[i + 1].trim() : '';
    if (!frontmatter) continue;

    let patterns = [];
    let tags = [];
    for (const line of frontmatter.split('\n')) {
      const trimmed = line.trim();
      if (trimmed.startsWith('pattern:')) {
        patterns = splitList(trimmed.substring('pattern:'.length));
      }
      if (trimmed.startsWith('tag:')) {
        // A '#' typed out of habit is not part of the tag, as in the tags property
        tags = splitList(trimmed.substring('tag:'.length))
          .map((t) => t.replace(/^#+/, '').trim())
          .filter((t) => t);
      }
    }
    if ((patterns.length > 0 || tags.length > 0) && body) {
      results.push({ patterns, tags, body });
    }
  }
  return results;
}

function splitList(text) {
  return text
    .split(',')
    .map((p) => p.trim())
    .filter((p) => p);
}

function matchesPattern(pattern, page) {
  const { fileName, pageType } = page;
  const isDate = /^\d{4}-\d{2}-\d{2}$/.test(fileName);
  if (pattern === 'daily') return isDate;
  if (DAYS.includes(pattern)) {
    // UTC, so the day does not depend on the time zone of the server
    return isDate && DAYS[new Date(fileName + 'T00:00:00Z').getUTCDay()] === pattern;
  }
  if (pattern === 'weekly') return /^\d{4}-W\d{2}$/.test(fileName);
  if (pattern === 'monthly') return /^\d{4}-\d{2}$/.test(fileName);
  if (pattern === 'yearly') return /^\d{4}$/.test(fileName);
  if (pattern === 'page' || pattern === 'doc') return pageType === 'doc';
  if (pattern === 'journal') return pageType === 'journal';
  // A regular expression: /pattern/
  if (pattern.startsWith('/') && pattern.endsWith('/') && pattern.length > 2) {
    try {
      return new RegExp(pattern.slice(1, -1)).test(fileName);
    } catch (e) {
      return false;
    }
  }
  return false;
}

/** Puts the items of the templates at the end of the page, one after another. */
function insertTemplates(templates) {
  let previous = null;
  for (const template of templates) {
    for (const item of peta.parseItems(template.body)) {
      previous = peta.createItem({
        position: previous ? 'after:' + previous : 'last',
        level: item.level,
        body: item.body,
        transclude: item.transclude,
      });
    }
  }
}
