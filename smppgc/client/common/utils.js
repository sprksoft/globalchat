const LOG = localStorage.getItem("LOG") == "true";

export function log(v){
  if (LOG){
    console.log(v)
  }
}

export function setChild(element, child) {
  while (element.firstChild) {
    element.firstChild.remove();
  }
  element.appendChild(child);
}

// Er is geen betere manier om dit te doen denk ik.
export function has_virtkb(){
  return /Mobi|Android|iPad|iPhone|Tablet|Touch/i.test(navigator.userAgent);
}
