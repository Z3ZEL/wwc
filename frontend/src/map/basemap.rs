//! The map background (ARCHITECTURE §5.5, ADR 0024): Mapbox tiles when the build has a public
//! token (`WWC_MAPBOX_TOKEN`), with an Outdoors layer and an Aerial one; OpenStreetMap's own
//! tiles otherwise (dev, forks), with no choice of layer. Pure, so the tile URLs and the
//! attribution are tested on the host; `view` hands them to walkers.

use crate::config;

/// The layer the visitor picked with the layer button (memory only: every visit opens on
/// Outdoors). Builds without a Mapbox token have one layer and ignore it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Basemap {
    /// Trails, contour lines, forests and water: a map for the outdoors.
    #[default]
    Outdoors,
    /// Satellite and aerial imagery, with roads and place names on top.
    Aerial,
}

impl Basemap {
    /// What the layer button switches to.
    pub fn other(self) -> Self {
        match self {
            Self::Outdoors => Self::Aerial,
            Self::Aerial => Self::Outdoors,
        }
    }

    /// Hover text of the layer button.
    pub fn switch_hint(self) -> &'static str {
        match self.other() {
            Self::Outdoors => "Show the outdoor map",
            Self::Aerial => "Show aerial imagery",
        }
    }

    /// Mapbox style id. <https://docs.mapbox.com/api/maps/styles/#classic-mapbox-styles>
    fn mapbox_style(self) -> &'static str {
        match self {
            Self::Outdoors => "outdoors-v12",
            Self::Aerial => "satellite-streets-v12",
        }
    }
}

/// Where the tiles come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    /// OpenStreetMap's public tile server: fine for dev, not for production (its usage policy).
    Osm,
    /// Mapbox Static Tiles API, with a public token.
    Mapbox { token: &'static str },
}

/// One link of the attribution in the map footer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Credit {
    pub text: &'static str,
    pub url: &'static str,
}

const fn credit(text: &'static str, url: &'static str) -> Credit {
    Credit { text, url }
}

const OSM: Credit = credit("© OpenStreetMap contributors", "https://www.openstreetmap.org/copyright");
/// <https://docs.mapbox.com/help/dive-deeper/attribution/>: these links, plus the Mapbox
/// wordmark, and Maxar for the satellite styles.
const MAPBOX: Credit = credit("© Mapbox", "https://www.mapbox.com/about/maps");
const MAPBOX_OSM: Credit = credit("© OpenStreetMap", "https://www.openstreetmap.org/copyright");
const MAXAR: Credit = credit("© Maxar", "https://www.maxar.com/");
const FEEDBACK: Credit = credit("Improve this map", "https://apps.mapbox.com/feedback/");

impl Provider {
    /// Mapbox when the build has a public token, OpenStreetMap otherwise.
    pub fn configured() -> Self {
        config::mapbox_token().map_or(Self::Osm, |token| Self::Mapbox { token })
    }

    /// Whether the visitor can switch layers (the layer button is shown).
    pub fn has_layers(self) -> bool {
        matches!(self, Self::Mapbox { .. })
    }

    /// Mapbox requires its wordmark on the map, next to the attribution (`ui::footer`).
    pub fn has_wordmark(self) -> bool {
        matches!(self, Self::Mapbox { .. })
    }

    /// Whose servers the browser loads the map images from (privacy notice).
    pub fn name(self) -> &'static str {
        match self {
            Self::Osm => "OpenStreetMap",
            Self::Mapbox { .. } => "Mapbox",
        }
    }

    /// Side of a tile image in points: walkers draws 512 px tiles as one 256 px tile of the
    /// zoom level below, so Mapbox needs a quarter of the requests.
    pub fn tile_size(self) -> u32 {
        match self {
            Self::Osm => 256,
            Self::Mapbox { .. } => 512,
        }
    }

    /// `hidpi`: a screen with more than one pixel per point gets Mapbox's @2x tiles (1024 px),
    /// so labels stay sharp. OpenStreetMap has no such tiles.
    pub fn tile_url(self, basemap: Basemap, zoom: u8, x: u32, y: u32, hidpi: bool) -> String {
        match self {
            Self::Osm => format!("https://tile.openstreetmap.org/{zoom}/{x}/{y}.png"),
            Self::Mapbox { token } => format!(
                "https://api.mapbox.com/styles/v1/mapbox/{}/tiles/512/{zoom}/{x}/{y}{}?access_token={token}",
                basemap.mapbox_style(),
                if hidpi { "@2x" } else { "" },
            ),
        }
    }

    /// The attribution shown in the map footer, in order.
    pub fn credits(self, basemap: Basemap) -> &'static [Credit] {
        match (self, basemap) {
            (Self::Osm, _) => &[OSM],
            (Self::Mapbox { .. }, Basemap::Outdoors) => &[MAPBOX, MAPBOX_OSM, FEEDBACK],
            (Self::Mapbox { .. }, Basemap::Aerial) => &[MAPBOX, MAPBOX_OSM, MAXAR, FEEDBACK],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WITH_TOKEN: Provider = Provider::Mapbox { token: "pk.test" };

    #[test]
    fn the_layer_button_switches_between_two_layers() {
        assert_eq!(Basemap::default(), Basemap::Outdoors, "visits open on the outdoor map");
        assert_eq!(Basemap::Outdoors.other(), Basemap::Aerial);
        assert_eq!(Basemap::Aerial.other(), Basemap::Outdoors);
        assert_eq!(Basemap::Outdoors.switch_hint(), "Show aerial imagery");
        assert_eq!(Basemap::Aerial.switch_hint(), "Show the outdoor map");
        assert!(WITH_TOKEN.has_layers());
        assert!(!Provider::Osm.has_layers(), "OpenStreetMap has no aerial imagery");
    }

    #[test]
    fn mapbox_tile_urls() {
        assert_eq!(
            WITH_TOKEN.tile_url(Basemap::Outdoors, 7, 64, 45, false),
            "https://api.mapbox.com/styles/v1/mapbox/outdoors-v12/tiles/512/7/64/45?access_token=pk.test"
        );
        assert_eq!(
            WITH_TOKEN.tile_url(Basemap::Aerial, 12, 2100, 1460, true),
            "https://api.mapbox.com/styles/v1/mapbox/satellite-streets-v12/tiles/512/12/2100/1460@2x?access_token=pk.test"
        );
        assert_eq!(WITH_TOKEN.tile_size(), 512);
    }

    #[test]
    fn without_a_token_the_map_is_openstreetmap() {
        for basemap in [Basemap::Outdoors, Basemap::Aerial] {
            assert_eq!(Provider::Osm.tile_url(basemap, 7, 64, 45, true), "https://tile.openstreetmap.org/7/64/45.png");
            assert_eq!(Provider::Osm.credits(basemap), [OSM]);
        }
        assert_eq!(Provider::Osm.tile_size(), 256);
        assert_eq!(Provider::Osm.name(), "OpenStreetMap");
        assert!(!Provider::Osm.has_wordmark() && WITH_TOKEN.has_wordmark());
        assert_eq!(Provider::configured().has_layers(), config::mapbox_token().is_some());
    }

    #[test]
    fn mapbox_credits_follow_its_attribution_rules() {
        let texts = |b| WITH_TOKEN.credits(b).iter().map(|c| c.text).collect::<Vec<_>>();
        assert_eq!(texts(Basemap::Outdoors), ["© Mapbox", "© OpenStreetMap", "Improve this map"]);
        assert_eq!(texts(Basemap::Aerial), ["© Mapbox", "© OpenStreetMap", "© Maxar", "Improve this map"]);
        for basemap in [Basemap::Outdoors, Basemap::Aerial] {
            assert!(WITH_TOKEN.credits(basemap).iter().all(|c| c.url.starts_with("https://")));
        }
    }
}
