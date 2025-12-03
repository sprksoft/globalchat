export enum ProtoError {
  Ok = "",
  Unexpected = "INT: unexpected",
  Kick = "err_kick",
  RateLimit = "err_ratelimit",
  IPRateLimit = "err_ipratelimit",
  ChatFull = "err_full",
  Shutdown = "err_shutdown",
  AlreadyInChat = "err_already_in_chat",
  NoSession = "err_no_session",
  Banned = "err_banned",

  UsernameInvalid = "err_username_invalid",
  UsernameLength = "err_username_length",
  UsernameTaken = "err_username_taken",
  UsernameProf = "err_username_prof",
  Disclaimer = "err_disclaimer",

  Protocol = "err_protocol",
  Retry = "retry",
}

const ERRORS: any = {
  err_kick: "Je bent gekicked uit de chat.",
  err_ratelimit:
    "Te veel berichten. Typ langzamer.\nJe kunt terug joinen binnen een paar seconden",
  err_ipratelimit: "Er zijn spammers met het zelfde ip als jou.",
  err_505: "Stop and wait a sec When you look at me like that, my darlin', what did you expect? I'd probably still adore you with your hands around my neck",
  err_full: "De chat zit vol.",
  err_shutdown: "De server wordt herstart. Sorry voor het ongemak.",
  err_already_in_chat:
    "Je bent al in de chat op een anderen tab of een ander apparaat.",
  err_no_session:
    "Je bent nog niet gelinked met smartschool. Ga terug naar de start pagina.",

  err_username_invalid: "Gebruikersnaam bevat ongeldige letters.",
  err_username_length: "Gebruikersnaam is te kort of te lang.",
  err_username_taken: "Gebruikersnaam is bezet.",
  err_username_prof: "Gebruikersnaam bevat mogelijks een ongepast woord",
};

export namespace ProtoError {
  export function humanize(protoerr: ProtoError): string {
    if (
      protoerr == ProtoError.RateLimit &&
      Math.floor(Math.random() * 505) == 1
    ) {
      return ERRORS["err_505"];
    }
    let herr = ERRORS[protoerr as string];
    if (!herr) {
      herr = "Onverwachte fout.";
    }
    return herr;
  }
}
