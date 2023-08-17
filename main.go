package main

func main() {
	clip, err := NewClipboard()
	if err != nil {
		ErrorExit(err)
	}

	clip.Run()
}
