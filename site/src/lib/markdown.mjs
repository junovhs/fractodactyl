import { marked } from 'marked';
import sanitize from 'sanitize-html';

// Release notes are remote Markdown; never put executable HTML into the site.
export function renderMarkdown(source) {
  return sanitize(marked.parse(source, { async: false }), {
    allowedTags: sanitize.defaults.allowedTags,
    allowedAttributes: { a: ['href', 'title'], code: ['class'] },
    allowedSchemes: ['https', 'http', 'mailto'],
  });
}
