use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Hash, std::cmp::Eq)]
#[allow(dead_code)]
pub enum PropDef {
    GuiWindowWidth = 0,
    GuiWindowHeight,
    GuiWindowTitle,
    GuiPane1Pos,
    GuiPane2Pos,
    GuiCol1Width,
    GuiFontSizeManualEnable,
    GuiFontSizeManual,
    GuiList0SortColumn,
    GuiList0SortAscending,
    AppUrl,
    BrowserDir,
    BrowserBackgroundLevel,
    BrowserClearCache,
    BrowserZoomPercent,
    AppRcsVersion,
    AppModeDebug,
    SystrayEnable,
}

#[allow(dead_code)]
pub const PROPDEF_ARRAY: [PropDef; 18] = [
    PropDef::GuiWindowWidth,
    PropDef::GuiWindowHeight,
    PropDef::GuiWindowTitle,
    PropDef::GuiPane1Pos,
    PropDef::GuiPane2Pos,
    PropDef::GuiCol1Width,
    PropDef::GuiFontSizeManualEnable,
    PropDef::GuiFontSizeManual,
    PropDef::GuiList0SortColumn,
    PropDef::GuiList0SortAscending,
    PropDef::AppUrl,
    PropDef::BrowserDir,
    PropDef::BrowserBackgroundLevel,
    PropDef::BrowserClearCache,
    PropDef::BrowserZoomPercent,
    PropDef::AppRcsVersion,
    PropDef::AppModeDebug,
	PropDef::SystrayEnable,
];

impl FromStr for PropDef {
    type Err = ();

    fn from_str(input: &str) -> Result<PropDef, Self::Err> {
        match input {
            "GuiWindowWidth" => Ok(PropDef::GuiWindowWidth),
            "GuiWindowHeight" => Ok(PropDef::GuiWindowHeight),
            "GuiWindowTitle" => Ok(PropDef::GuiWindowTitle),
            "GuiPane1Pos" => Ok(PropDef::GuiPane1Pos),
            "GuiPane2Pos" => Ok(PropDef::GuiPane2Pos),
            "GuiCol1Width" => Ok(PropDef::GuiCol1Width),
            "GuiFontSizeManualEnable" => Ok(PropDef::GuiFontSizeManualEnable),
            "GuiFontSizeManual" => Ok(PropDef::GuiFontSizeManual),
            "GuiList0SortColumn" => Ok(PropDef::GuiList0SortColumn),
            "GuiList0SortAscending" => Ok(PropDef::GuiList0SortAscending),
            "AppUrl" => Ok(PropDef::AppUrl),
            "BrowserDir" => Ok(PropDef::BrowserDir),
            "BrowserBackgroundLevel" => Ok(PropDef::BrowserBackgroundLevel),
            "BrowserZoomPercent" => Ok(PropDef::BrowserBackgroundLevel),
            "AppRcsVersion" => Ok(PropDef::AppRcsVersion),
            "AppModeDebug" => Ok(PropDef::AppModeDebug),
            _ => Err(()),
        }
    }
}

impl PropDef {
    pub fn tostring(&self) -> String {
        format!("{:?}", &self)
    }
}

impl std::fmt::Display for PropDef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub struct FontAttributes {}
impl FontAttributes {
    const BITMASK_ISTRANSPARENT: u32 = 1024;
    const BITMASK_ISFOLDER: u32 = 512;
    const BITMASK_ISREAD: u32 = 256;
    const BITMASK_FONTSIZE: u32 = 255;

    pub fn to_activation_bits(
        fontsize: u32,
        is_read: bool,
        is_folder: bool,
        transparent: bool,
    ) -> u32 {
        (fontsize & Self::BITMASK_FONTSIZE)
            | match is_read {
                true => Self::BITMASK_ISREAD,
                _ => 0,
            }
            | match is_folder {
                true => Self::BITMASK_ISFOLDER,
                _ => 0,
            }
            | match transparent {
                true => Self::BITMASK_ISTRANSPARENT,
                _ => 0,
            }
    }

    /// returns  font_size, is_read, is_folder, transparent
    pub fn from_activation_bits(bits: u32) -> (u32, bool, bool, bool) {
        (
            (bits & Self::BITMASK_FONTSIZE),
            ((bits & Self::BITMASK_ISREAD) > 0),
            ((bits & Self::BITMASK_ISFOLDER) > 0),
            ((bits & Self::BITMASK_ISTRANSPARENT) > 0),
        )
    }
}
