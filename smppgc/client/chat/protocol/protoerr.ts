export enum ProtoError {
  ok = "",
  err_kick = "err_kick",
  err_ratelimit = "err_ratelimit",
  err_ipratelimit = "err_ipratelimit",
  err_505 = "err_505",
  err_full = "err_full",
  err_shutdown = "err_shutdown",
  err_already_in_chat = "err_already_in_chat",
  err_no_session = "err_no_session ",
  err_banned = "err_banned",

  err_username_invalid = "err_username_invalid",
  err_username_length = "err_username_length",
  err_username_taken = "err_username_taken",
  err_username_prof = "err_username_prof",

  retry = "retry",
}

const ERRORS: any = {
  err_kick: "Je bent uit de chat gezet door een admin.",
  err_ratelimit:
    "Te veel berichten. Typ langzamer.\nJe kunt terug joinen binnen een paar seconden",
  err_ipratelimit: "Er zijn spammers met het zelfde ip als jou.",
  err_505:
    "Stop and wait a sec when you look at me like that my darling what do you expect. In my imagination you're waiting lying on your side with your hands between your theights.",
  err_full: "De chat zit vol.",
  err_shutdown: "De server wordt herstart. Sorry voor het ongemak.",
  err_already_in_chat:
    "Je bent al in de chat op een anderen tab of een ander aparaat.",
  err_no_session:
    "Je bent nog niet gelinked met smartschool. Ga terug naar de start pagina.",

  err_username_invalid: "Gebruikersnaam bevat ongeldige letters.",
  err_username_length: "Gebruikersnaam is te kort of te lang.",
  err_username_taken: "Gebruikersnaam is bezet.",
  err_username_prof: "Gebruikersnaam is ongepast",
};

export namespace ProtoError {
  export function humanize(protoerr: ProtoError) {
    if (protoerr == ProtoError.err_ratelimit && Math.floor(Math.random() * 505) == 1) {
      return ProtoError.err_505;
    }
    let herr = ERRORS[protoerr as string];
    if (!herr) {
      herr = "Onverwachte fout.";
    }
    return herr;
  }
}
