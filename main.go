package main

import "fmt"

func main() {
	redis, err := NewRedis()
	if err != nil {
		ErrorExit(err)
	}

	ch := redis.Sub()
	for frame := range ch {
		if frame.Type == DataFrameText {
			fmt.Printf("Text: %s\n", string(frame.Data))
		}
	}
}
