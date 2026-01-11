const LOG: boolean = localStorage.getItem("LOG") == "true";

export function log(v: string) {
  if (LOG) {
    console.log(v);
  }
}

export function getCSRFToken(): string {
  const token = document.cookie
    .split("; ")
    .find((c) => c.startsWith("csrf-protect="))
    ?.split("=")[1];

  return token ? token : ""
}

// Er is geen betere manier om dit te doen denk ik.
export function hasVirtKb(): boolean {
  return /Mobi|Android|iPad|iPhone|Tablet|Touch/i.test(navigator.userAgent);
}
