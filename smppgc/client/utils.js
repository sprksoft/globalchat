const LOG = localStorage.getItem("LOG") == "true";

export function log(v){
  if (LOG){
    console.log(v)
  }
}
