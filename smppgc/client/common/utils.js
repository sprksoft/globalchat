const LOG = localStorage.getItem("LOG") == "true";

export function log(v) {
  if (LOG) {
    console.log(v);
  }
}

export function getCSRFToken() {
  return document.cookie
    .split("; ")
    .find((c) => c.startsWith("csrf-protect="))
    ?.split("=")[1];
}

// Er is geen betere manier om dit te doen denk ik.
export function hasVirtKb() {
  return /Mobi|Android|iPad|iPhone|Tablet|Touch/i.test(navigator.userAgent);
}
