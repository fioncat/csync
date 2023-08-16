package main

import (
	"reflect"
	"testing"
)

func TestDataFrame(t *testing.T) {
	dataList := []DataFrame{
		{
			Type: DataFrameImage,
			From: "user-01",
			Data: []byte{2, 3, 4, 12, 43, 54, 12},
		},
		{
			Type: DataFrameText,
			From: "user-02",
			Data: []byte("Test hello world!"),
		},
		{
			Type: DataFrameText,
			From: "user-03",
			Data: []byte(""),
		},
		{
			Type: DataFrameText,
			From: "user-03",
			Data: []byte("a"),
		},
		{
			Type: DataFrameImage,
			From: "user-04",
			Data: []byte{},
		},
		{
			Type: DataFrameImage,
			From: "user-04",
			Data: []byte{6},
		},
	}

	for _, f := range dataList {
		data := f.Encode()

		expected, err := DecodeDataFrame(data, f.From)
		if err != nil {
			t.Fatal(err)
		}

		if !reflect.DeepEqual(expected, &f) {
			t.Fatalf("Unexpect frame: %+v", expected)
		}

	}
}

func TestPassword(t *testing.T) {
	testCases := []struct {
		password string
		data     []byte
	}{
		{
			password: "test password",
			data:     []byte{1, 2, 3, 4},
		},
		{
			password: "hello world!",
			data:     []byte{1},
		},
		{
			password: "xxxxx",
			data:     []byte{},
		},
		{
			password: "xxx66688**",
			data:     []byte{78, 12, 32, 4, 12, 54},
		},
		{
			password: "@@@@",
			data:     []byte{66, 44, 11},
		},
	}

	for _, testCase := range testCases {
		f, err := newPasswordFormatter(testCase.password)
		if err != nil {
			t.Fatal(err)
		}
		message, err := f.Encode(testCase.data)
		if err != nil {
			t.Fatal(err)
		}

		expectData, err := f.Decode(message)
		if err != nil {
			t.Fatal(err)
		}

		if len(expectData) == 0 && len(testCase.data) == 0 {
			continue
		}

		if !reflect.DeepEqual(testCase.data, expectData) {
			t.Fatalf("Unexpect decode: %v", expectData)
		}
	}
}
