export enum PacketId {
  MESSAGE = 0,
  //UNUSED = 1,
  MESSAGE_SYSTEM = 2,
  SETUP = 3,
  USERJOIN = 4,
  MODJOIN = 5,
  //UNUSED = 6,
  MESSAGE_DEL = 7,
  //UNUSED = 8,
  USER_COUNT = 9,
}
export enum PacketC2SId {
  MESSAGE = 0,
  DELMSG = 1,
  BANMSGAUTHOR = 2,

  WF_MARKGOOD = 3,
  WF_MARKBAD = 4,
  WF_COMMIT = 5,
  WF_LOCK = 6,
  WF_UNLOCK = 7,

  REPORT = 8,
}
