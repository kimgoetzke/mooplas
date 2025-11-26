- Don't remove existing comments from any file you're editing or suggestions you're making unless they directly relate
  to your change.
- Don't remove docs from any file you're editing or suggestions you're making unless they directly relate to your
  change.
- Don't ever repeat the entire code of a file back to the user unless you change more than 95% of lines of code in a
  file.
- Don't abbreviate variable, parameter, and field names unnecessarily e.g. don't use `pos` for `position`.
- Functions that are Bevy ECS systems should have a name ending in `_system`.
- Functions that handle Bevy `Message`s should have a name starting with `handle_` and ending in `_message`.