//! Formiga Home: an optional dollhouse beside Formiga Desktop, where the owner steps into one
//! household's house, arranges it, shows the things the colony has found, and spends a little
//! time with the residents.

mod actor;
mod app;
mod arrange;
mod art;
mod catalog;
mod character;
mod host;
mod house;
mod household;
mod icon;
mod iso;
mod journal;
mod keepsakes;
mod life;
mod paint;
mod path;
mod room;
mod scene;
mod session;
mod staging;
mod starter;
mod store;

use anyhow::{Context, Result, bail};
use formiga_art::Canvas;
use household::Household;
use std::path::{Path, PathBuf};

const USAGE: &str = "\
Usage: formiga-home [--sample | --formiga-home <VISIT DIRECTORY> | --from-save <FILE> [--house <N>]]
                    [--render-room <PNG> | --render-catalog <PNG> | --render-finds <PNG>]

  --sample                 Open Desktop's sample household (the default): a rehearsal, kept in
                           Home's own data folder as Desktop would keep it
  --formiga-home <DIR>     How Desktop opens a house: the visit it wrote, answered on leaving
  --from-save <FILE>       Development only: rehearse a house in a Desktop colony file, which is
                           only ever read
  --house <N>              Which house to open, counting the colony house as 0
  --render-room <PNG>      Draw the house to a PNG and exit without opening a window
  --lived-in               With --render-room, or a rehearsal with no house yet: the house a few
                           weeks on, with finds and keepsakes shown
  --floor <ID> --wall <ID> With --lived-in: the finishes to draw it in
  --render-catalog <PNG>   Draw every piece of furniture at every turn, for review
  --render-poses <PNG>     Draw everyone in every pose Home uses, for review
  --render-finds <PNG>     Draw everything the colony has, every way it can be shown, for review
  --at <SECONDS>           With --render-room: the household's own life that far in
  --rooms <N>              With --render-room or a rehearsal: the house grown to N rooms (up to 3)
  --snap <PNG>             Open the window, and after --at seconds (3 if not given) save a
                           picture of the window itself and close, for review
  --page <PAGE>            With --snap: open arranging, on finds, furniture or rooms; or the
                           journal
  --theme <THEME>          With --snap: the notebook light or dark, whatever the household's
                           own preference
  --zoom <STEPS>           With --snap: the house that many whole pixels closer than fits
  --scale <N>              Pixels per scene pixel in a PNG (default 3)
  --home-version           Print the newest Home version this build reads, for packaging
  --icon <FOLDER>          Write Home's icon as .icns, .ico and .png, for packaging
";

enum Render {
    Room,
    Poses,
    Catalog,
    Finds,
}

enum Source {
    Sample,
    Visit(PathBuf),
    Save(PathBuf),
}

struct Args {
    source: Source,
    house: usize,
    render: Option<(Render, PathBuf)>,
    lived_in: bool,
    floor: String,
    wall: String,
    scale: u32,
    at: Option<f32>,
    rooms: usize,
    snap: Option<PathBuf>,
    page: Option<String>,
    theme: Option<String>,
    zoom: i32,
}

fn parse_args(mut args: impl Iterator<Item = std::ffi::OsString>) -> Result<Option<Args>> {
    let mut parsed = Args {
        source: Source::Sample,
        house: 0,
        render: None,
        lived_in: false,
        floor: "floor.boards".to_owned(),
        wall: "wall.leafy".to_owned(),
        scale: 3,
        at: None,
        rooms: 1,
        snap: None,
        page: None,
        theme: None,
        zoom: 0,
    };
    let value = |args: &mut dyn Iterator<Item = std::ffi::OsString>, flag: &str| {
        args.next()
            .map(PathBuf::from)
            .with_context(|| format!("{flag} needs a value"))
    };
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("--help" | "-h") => return Ok(None),
            Some("--sample") => parsed.source = Source::Sample,
            Some(formiga_home_contract::LAUNCH_ARGUMENT) => {
                parsed.source =
                    Source::Visit(value(&mut args, formiga_home_contract::LAUNCH_ARGUMENT)?);
            }
            Some("--from-save") => parsed.source = Source::Save(value(&mut args, "--from-save")?),
            Some("--house") => {
                parsed.house = value(&mut args, "--house")?
                    .to_string_lossy()
                    .parse()
                    .context("--house is a number")?;
            }
            Some("--render-room") => {
                parsed.render = Some((Render::Room, value(&mut args, "--render-room")?));
            }
            Some("--render-catalog") => {
                parsed.render = Some((Render::Catalog, value(&mut args, "--render-catalog")?));
            }
            Some("--render-poses") => {
                parsed.render = Some((Render::Poses, value(&mut args, "--render-poses")?));
            }
            Some("--render-finds") => {
                parsed.render = Some((Render::Finds, value(&mut args, "--render-finds")?));
            }
            Some("--lived-in") => parsed.lived_in = true,
            Some("--floor") => parsed.floor = value(&mut args, "--floor")?.to_string_lossy().into(),
            Some("--wall") => parsed.wall = value(&mut args, "--wall")?.to_string_lossy().into(),
            Some("--at") => {
                parsed.at = Some(
                    value(&mut args, "--at")?
                        .to_string_lossy()
                        .parse()
                        .context("--at is a number of seconds")?,
                );
            }
            Some("--snap") => parsed.snap = Some(value(&mut args, "--snap")?),
            Some("--zoom") => {
                parsed.zoom = value(&mut args, "--zoom")?
                    .to_string_lossy()
                    .parse()
                    .context("--zoom is a number of steps")?;
            }
            Some("--theme") => {
                parsed.theme = Some(value(&mut args, "--theme")?.to_string_lossy().into());
            }
            Some("--page") => {
                parsed.page = Some(value(&mut args, "--page")?.to_string_lossy().into());
            }
            Some("--rooms") => {
                parsed.rooms = value(&mut args, "--rooms")?
                    .to_string_lossy()
                    .parse()
                    .context("--rooms is a number")?;
            }
            Some("--scale") => {
                parsed.scale = value(&mut args, "--scale")?
                    .to_string_lossy()
                    .parse()
                    .context("--scale is a number")?;
            }
            _ => bail!("unexpected argument {}\n\n{USAGE}", arg.to_string_lossy()),
        }
    }
    Ok(Some(parsed))
}

fn main() -> Result<()> {
    // Before anything else starts a thread, while the clock's offset can be read soundly.
    journal::read_local_offset();
    // For the packaging scripts: the newest Home version this build reads, which goes in the
    // macOS bundle and the Windows registry for Desktop to find.
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--home-version")
    {
        println!("{}", formiga_home_contract::HOME_FORMAT_VERSION);
        return Ok(());
    }
    // For the packaging scripts too: Home's icon, for the bundle and the installer.
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--icon")
    {
        let folder = std::env::args_os()
            .nth(2)
            .map(PathBuf::from)
            .context("--icon needs a folder to write the icon into")?;
        std::fs::create_dir_all(&folder)?;
        std::fs::write(folder.join("FormigaHome.icns"), icon::icns())?;
        std::fs::write(folder.join("FormigaHome.ico"), icon::ico())?;
        std::fs::write(folder.join("FormigaHome.png"), icon::png(&icon::at(1024)))?;
        println!("Wrote Home's icon to {}", folder.display());
        return Ok(());
    }
    let Some(args) = parse_args(std::env::args_os().skip(1))? else {
        print!("{USAGE}");
        return Ok(());
    };
    if let Some((render, path)) = &args.render {
        let snapshot = match &args.source {
            Source::Sample => formiga_home_contract::sample::snapshot(),
            Source::Save(save) => {
                host::Colony::Save(save.clone()).open(host::Which::Nth(args.house))?
            }
            Source::Visit(_) => {
                bail!("a visit from Desktop opens a window; draw the sample or a colony file")
            }
        };
        let household = Household::new(snapshot).context("could not draw the household")?;
        let canvas = render_to(render, &household, &args);
        write_png(path, &canvas, args.scale)?;
        println!("Drew {} to {}", household.house_name(), path.display());
        return Ok(());
    }

    // One house at a time: see `store::take`. A visit that arrives while another window is open
    // is answered as busy.
    let data = store::folder();
    let open = data.as_deref().map(store::take);
    let busy = matches!(open, Some(Err(store::Busy)));
    let (host, household) = match args.source {
        Source::Visit(dir) => {
            let (visit, household) = session::arrive(&dir, busy)?;
            (host::Host::Visit(visit), household)
        }
        source => {
            if busy {
                bail!("Formiga Home is already open, with a house open in it");
            }
            let (colony, label) = match source {
                Source::Save(save) => (
                    host::Colony::Save(save),
                    "Rehearsing a colony file".to_owned(),
                ),
                _ => (
                    host::Colony::Sample,
                    "Rehearsing Desktop's sample colony".to_owned(),
                ),
            };
            let snapshot = colony.open(host::Which::Nth(args.house))?;
            let household =
                Household::new(snapshot.clone()).context("could not draw the household")?;
            let mut homes = store::RehearsalHomes::new(data.as_deref(), &snapshot.colony_key);
            if args.lived_in {
                // For review: a house that has been lived in, if the rehearsal has none yet.
                homes.live_in(&household, &args.floor, &args.wall);
            }
            if args.rooms > 1 {
                // For review: the rehearsal's house grown, as if the owner had built on.
                homes.grow(&snapshot, args.rooms);
            }
            (
                host::Host::Rehearsal(host::Rehearsal::new(snapshot, homes, label, colony)),
                household,
            )
        }
    };
    let open = open.and_then(Result::ok);
    let place = data.as_deref().and_then(store::WindowPlace::load);
    let mut viewport = app::frameless(eframe::egui::ViewportBuilder::default())
        .with_title(format!("Formiga Home \u{2014} {}", household.house_name()))
        .with_inner_size(place.map_or([840.0, 620.0], |place| [place.width, place.height]))
        .with_min_inner_size([640.0, 480.0])
        .with_icon({
            let picture = icon::at(64);
            eframe::egui::IconData {
                rgba: picture.rgba_bytes(),
                width: 64,
                height: 64,
            }
        });
    if let Some(place) = place {
        viewport = viewport.with_position([place.x, place.y]);
    }
    if args.snap.is_some() {
        // A window for review only: it stays behind whatever the owner is doing.
        viewport = viewport.with_active(false);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Formiga Home",
        options,
        Box::new(move |cc| {
            let mut app = app::HomeApp::new(&cc.egui_ctx, household, host, data, open);
            if let Some(path) = args.snap {
                app.snap(
                    path,
                    args.at.unwrap_or(3.0),
                    args.page.as_deref(),
                    args.zoom,
                );
                match args.theme.as_deref() {
                    Some("dark") => cc.egui_ctx.set_theme(eframe::egui::ThemePreference::Dark),
                    Some("light") => cc.egui_ctx.set_theme(eframe::egui::ThemePreference::Light),
                    _ => {}
                }
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|error| anyhow::anyhow!("the window could not open: {error}"))
}

/// One of the review pictures.
fn render_to(render: &Render, household: &Household, args: &Args) -> Canvas {
    match render {
        Render::Catalog => art::furniture::sheet(),
        Render::Poses => staging::poses(household),
        Render::Finds => {
            // Everything the colony has, then what a lived-in house has made of its own.
            let home = staging::lived_in(household, &args.floor, &args.wall);
            let mut snapshot = household.snapshot.clone();
            keepsakes::stock(&mut snapshot, &home);
            let pictures = keepsakes::pictures(&home);
            art::displays::sheet(&snapshot.inventory, |item| {
                pictures
                    .get(&item.id)
                    .cloned()
                    .unwrap_or_else(|| art::displays::icon(item))
            })
        }
        Render::Room => {
            let home = if args.lived_in {
                staging::lived_in(household, &args.floor, &args.wall)
            } else {
                starter::home(&household.snapshot)
            };
            let mut home = staging::grown(home, args.rooms, &household.snapshot);
            arrange::settle(&mut home);
            let mut snapshot = household.snapshot.clone();
            keepsakes::stock(&mut snapshot, &home);
            let house = house::House::of(&home.rooms);
            let mut scene = scene::Scene::new(&house);
            scene.set_pictures(keepsakes::pictures(&home));
            let mut overlay = scene::Overlay {
                backdrop: true,
                ..scene::Overlay::default()
            };
            match args.at {
                // The household's own life, run forward as the window would run it.
                Some(until) => {
                    let mut life = life::Life::new(household, &house);
                    let mut now = 0.0;
                    while now < until {
                        now += 1.0 / 30.0;
                        life.tick(household, &house, &snapshot, &home.likings, now, 1.0 / 30.0);
                    }
                    for id in life.present() {
                        println!("{}", life.doing(household, &house, &snapshot, id));
                    }
                    let mut seen = house.clone();
                    let worn = life.worn();
                    seen.shown.retain(|shown| !worn.contains(&shown.item));
                    overlay.lamps_off = life.lamps_off().to_vec();
                    scene.compose(&seen, &snapshot, &mut life.actors, now, &overlay)
                }
                None => {
                    let mut actors = staging::pose(household, &house, household.reduce_motion());
                    scene.compose(&house, &snapshot, &mut actors, 0.5, &overlay)
                }
            }
        }
    }
}

/// A canvas as a PNG, each pixel `scale` pixels square.
pub(crate) fn write_png(path: &Path, canvas: &Canvas, scale: u32) -> Result<()> {
    let scale = scale.max(1);
    let (width, height) = (canvas.width() * scale, canvas.height() * scale);
    let mut bytes = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let pixel = canvas.get((x / scale) as i32, (y / scale) as i32);
            bytes.extend([pixel.r, pixel.g, pixel.b, pixel.a]);
        }
    }
    let file = std::fs::File::create(path)
        .with_context(|| format!("could not write {}", path.display()))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&bytes)?;
    Ok(())
}
