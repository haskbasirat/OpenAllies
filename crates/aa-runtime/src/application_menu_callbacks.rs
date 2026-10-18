//! Exact MainMenu/Loading OnEnter selection. The menu is a borrowed opaque
//! application allocation; actual presentation is a required child.
pub type Result<T> = std::result::Result<T, String>;
pub fn leave_loading_40caa0(changed_8: &mut u8) {
    *changed_8 = 0;
}
pub fn loading_finished_682db0(flag_15: &mut u8, argument: u8) {
    if argument == 0 {
        *flag_15 = 1;
    }
}
pub trait Services {
    type Menu;
    fn current_menu(&mut self) -> Result<Option<Self::Menu>>;
    fn show_5c0440(&mut self, menu: Self::Menu, animate: bool) -> Result<()>;
}
///40CA80: explicit nonnull argument wins; otherwise reread this+4.
pub fn enter<S: Services>(services: &mut S, explicit: Option<S::Menu>) -> Result<()> {
    let menu = match explicit {
        Some(menu) => Some(menu),
        None => services.current_menu()?,
    };
    if let Some(menu) = menu {
        services.show_5c0440(menu, true)?;
    }
    Ok(())
}
pub trait LoadingServices: Services {
    fn kind_10(&mut self) -> Result<u32>;
    fn splash_b39748(&mut self) -> Result<Option<Self::Menu>>;
    fn menu_table_2_40d0f0(&mut self) -> Result<Option<Self::Menu>>;
    fn store_menu_4(&mut self, menu: Option<Self::Menu>) -> Result<()>;
}
///682E70 ignores Arg8, selects only kind0/1, then passes retained+4 to40CA80.
pub fn enter_loading<S: LoadingServices>(services: &mut S) -> Result<()> {
    match services.kind_10()? {
        0 => {
            let menu = services.splash_b39748()?;
            services.store_menu_4(menu)?;
        }
        1 => {
            let menu = services.menu_table_2_40d0f0()?;
            services.store_menu_4(menu)?;
        }
        _ => {}
    }
    let explicit = services.current_menu()?;
    enter(services, explicit)
}
///6E8C1E..6E8CA9 after the modern interface update. Guards remain lazy;
///successful current status precedes showing the same ReadyButton.
pub trait ReadyServices {
 fn button_visible(&mut self)->Result<bool>;
 fn resource_complete_17(&mut self)->Result<bool>;
 fn user_ready_16(&mut self)->Result<bool>;
 fn finished_status_773ce0(&mut self)->Result<()>;
 fn show_ready_button(&mut self)->Result<()>;
}
pub fn ready_prompt_6e8c00(s:&mut impl ReadyServices)->Result<()>{
 if !s.button_visible()?&&s.resource_complete_17()?&&!s.user_ready_16()?{
  s.finished_status_773ce0()?;s.show_ready_button()?;
 }
 Ok(())
}
