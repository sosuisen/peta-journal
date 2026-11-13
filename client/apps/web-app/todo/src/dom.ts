/** Whether the element is a link or is inside one. */
export function isOnLink(element: {
  closest(selector: string): unknown;
}): boolean {
  return element.closest('a') !== null;
}
