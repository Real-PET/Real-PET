#pragma once
/**
 * @warning These have to be in order.
 * @see TextureResourceManager
*/
enum TextureType {
  //Tiles
  TILE_BLUE_NORMAL,
  TILE_BLUE_CRACKED,
  TILE_BLUE_BROKEN,
  TILE_BLUE_ICE,
  TILE_BLUE_GRASS,
  TILE_BLUE_PURPLE,
  TILE_BLUE_LAVA,
  TILE_BLUE_EMPTY,
  TILE_RED_NORMAL,
  TILE_RED_CRACKED,
  TILE_RED_BROKEN,
  TILE_RED_ICE,
  TILE_RED_GRASS,
  TILE_RED_PURPLE,
  TILE_RED_LAVA,
  TILE_RED_EMPTY,

  //Navis
  NAVI_MEGAMAN_ATLAS,
  NAVI_STARMAN_ATLAS,
  NAVI_ROLL_ATLAS,

  //Mobs
  MOB_MOVE,
  MOB_EXPLOSION,
  MOB_METTAUR,
  MOB_PROGSMAN_IDLE,
  MOB_PROGSMAN_MOVE,
  MOB_PROGSMAN_SHOOT,
  MOB_PROGSMAN_HIT,
  MOB_PROGSMAN_PUNCH,
  MOB_PROGSMAN_THROW,
  MOB_CANODUMB_ATLAS,
  MOB_ANYTHING_GOES,

  //Spells
  SPELL_BULLET_HIT,
  SPELL_BUSTER_CHARGE,
  //SPELL_CHARGED_BULLET_HIT,
  SPELL_WAVE,
  SPELL_PROG_BOMB,

  //Chip Select 
  CHIP_SELECT_MENU,
  CHIP_CURSOR_SMALL,
  CHIP_CURSOR_BIG,
  CHIP_CARDS,
  CHIP_ICONS,
  CHIP_NODATA,
  CHIP_SENDDATA,
  CHIP_MISSINGDATA,
  CHIP_LOCK,

  // In-battle signs
  PROGRAM_ADVANCE,
  BATTLE_START,
  ENEMY_DELETED, 

  // Battle Result
  BATTLE_RESULTS_FRAME,
  BATTLE_RESULTS_ZENNY,
  BATTLE_RESULTS_NODATA,

  // Folder View
  FOLDER_CHIP_HOLDER,
  FOLDER_DOCK,
  FOLDER_RARITY,
  FOLDER_SCROLLBAR,

  // Navi Select View
  GLOWING_PAD_ATLAS,
  GLOWING_PAD_BASE,
  GLOWING_PAD_BOTTOM,
  CHAR_NAME,
  CHAR_STAT,
  CHAR_ELEMENT,
  CHAR_INFO_BOX,
  SYMBOL_SLOTS,

  // Navigator and textbox
  MUG_NAVIGATOR,
  MUG_PROG,
  ANIMATED_TEXT_BOX,
  TEXT_BOX_NEXT_CURSOR,

  //Background/Foreground
  BACKGROUND_BLUE,
  TITLE_ANIM_CHAR,
  GAME_OVER,
  BATTLE_SELECT_BG,
  MAIN_MENU,
  MAIN_MENU_OW,
  MAIN_MENU_OW2,
  MAIN_MENU_ARROW,
  FOLDER_VIEW_BG,
  NAVI_SELECT_BG,

  // OVERWORLD
  OW_MR_PROG,
  OW_NUMBERMAN,

  // ICONS
  GAMEPAD_SUPPORT_ICON,
  MAIN_MENU_UI,
  ELEMENT_ICON,
  // SHADER TEXTURE MAPS
  DISTORTION_TEXTURE,
  HEAT_TEXTURE,
  NOISE_TEXTURE,
  NOISE_INVERT_TEXTURE,
  // EDITOR UI
  LIGHT,
  TEXTURE_TYPE_SIZE
};