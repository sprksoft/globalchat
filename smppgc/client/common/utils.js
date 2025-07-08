const LOG = localStorage.getItem("LOG") == "true";

export function log(v){
  if (LOG){
    console.log(v)
  }
}

// Er is geen betere manier om dit te doen denk ik.
export function hasVirtKb(){
  return /Mobi|Android|iPad|iPhone|Tablet|Touch/i.test(navigator.userAgent);
}
