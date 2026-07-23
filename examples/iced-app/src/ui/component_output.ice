app ComponentOutput

extern crate::backend
  component native_help(active:bool) -> bool

extern plugin_backend
  component borrowed_help(label:&str, active:&bool) -> bool

theme
  bg #111111
  fg #eeeeee
  primary #3366ff
  danger #cc3333

component PluginHelp(active:bool) -> bool
  extern native_help(active) -> emit _

component BorrowedPluginHelp(label:str, active:bool) -> bool
  extern borrowed_help(label, active) -> emit _

component NestedPluginHelp(active:bool) -> bool
  PluginHelp active=active -> emit _

state
  active = false

on changed(next)
  active = next

view
  col
    NestedPluginHelp active=active -> changed _
    BorrowedPluginHelp label="Borrowed plugin" active=active -> changed _
