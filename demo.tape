Output assets/demo.gif

Set FontSize 22
Set Width 1500
Set Height 900
Set Padding 20
Set Margin 40
Set MarginFill "#0d0d0d"
Set WindowBar Colorful
Set BorderRadius 10
Set Theme {
  "background": "#000000",
  "foreground": "#ffffff",
  "cursor": "#ffffff",
  "selection": "#585dec"
}

Set TypingSpeed 30ms
Set PlaybackSpeed 1.0

Hide
Sleep 1s
Type 'export PATH="$PWD/target/release:$PATH"' Enter
Type 'clear' Enter
Sleep 500ms
Show

Sleep 1s
Type "pik --file assets/fruits.txt --prompt 'Select a fruit:'" Enter
Sleep 1.5s
Down 2 Sleep 300ms
Down 1 Sleep 400ms
Up 1 Sleep 500ms
Enter
Sleep 2s

Ctrl+L
Sleep 500ms

Type "pik --file assets/cities.txt --prompt 'Pick a city:'" Enter
Sleep 1.5s
Down 4 Sleep 250ms
Down 2 Sleep 350ms
Up 1 Sleep 400ms
Enter
Sleep 2s

Ctrl+L
Sleep 500ms

Type 'echo -e "A\nB\nC" | pik --prompt "Pick one:"' Enter
Sleep 1.5s
Down 1 Sleep 500ms
Enter
Sleep 2s
