package main

import (
	"encoding"
	"encoding/base64"
	"reflect"
	"sort"
)

// typedTree convertit une valeur décodée par gophertunnel en arbre JSON sans perte :
//
//   - les compounds NBT (`map[string]any`) gardent le type de chaque valeur : `{"int": 0}`,
//     `{"byte_array": [...]}`, `{"list": [...]}`, `{"compound": {...}}`… ;
//   - les champs non exportés sont inclus, ce qui rend visible le contenu de `protocol.Optional`
//     (`{"set": true, "val": ...}`), affiché `{}` par encoding/json.
//
// Les noms de champs sont ceux des structures Go (comme encoding/json sans tag).
func typedTree(v reflect.Value) any {
	// Types à forme texte (UUID…) : même rendu qu'encoding/json.
	if v.IsValid() && v.CanInterface() && v.Kind() != reflect.Pointer && v.Kind() != reflect.Interface {
		if m, ok := v.Interface().(encoding.TextMarshaler); ok {
			if text, err := m.MarshalText(); err == nil {
				return string(text)
			}
		}
	}
	switch v.Kind() {
	case reflect.Invalid:
		return nil
	case reflect.Pointer:
		if v.IsNil() {
			return nil
		}
		return typedTree(v.Elem())
	case reflect.Interface:
		if v.IsNil() {
			return nil
		}
		return typedTree(v.Elem())
	case reflect.Struct:
		out := make(map[string]any, v.NumField())
		t := v.Type()
		for i := 0; i < v.NumField(); i++ {
			out[t.Field(i).Name] = typedTree(v.Field(i))
		}
		return out
	case reflect.Map:
		if v.Type().Key().Kind() == reflect.String && v.Type().Elem().Kind() == reflect.Interface {
			return map[string]any{"compound": nbtCompound(v)}
		}
		out := make(map[string]any, v.Len())
		iter := v.MapRange()
		for iter.Next() {
			out[toKey(iter.Key())] = typedTree(iter.Value())
		}
		return out
	case reflect.Slice, reflect.Array:
		if v.Type().Elem().Kind() == reflect.Uint8 {
			b := make([]byte, v.Len())
			for i := range b {
				b[i] = byte(v.Index(i).Uint())
			}
			return base64.StdEncoding.EncodeToString(b)
		}
		if v.Kind() == reflect.Slice && v.IsNil() {
			return []any{}
		}
		out := make([]any, v.Len())
		for i := range out {
			out[i] = typedTree(v.Index(i))
		}
		return out
	case reflect.Bool:
		return v.Bool()
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return v.Int()
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		return v.Uint()
	case reflect.Float32, reflect.Float64:
		return v.Float()
	case reflect.String:
		return v.String()
	default:
		return v.Type().String()
	}
}

func toKey(v reflect.Value) string {
	if v.Kind() == reflect.String {
		return v.String()
	}
	t := typedTree(v)
	if s, ok := t.(string); ok {
		return s
	}
	return reflect.ValueOf(t).String()
}

// nbtCompound convertit un compound NBT gophertunnel en clés triées et valeurs typées.
func nbtCompound(v reflect.Value) map[string]any {
	keys := make([]string, 0, v.Len())
	for _, k := range v.MapKeys() {
		keys = append(keys, k.String())
	}
	sort.Strings(keys)
	out := make(map[string]any, len(keys))
	for _, k := range keys {
		out[k] = nbtValue(v.MapIndex(reflect.ValueOf(k)))
	}
	return out
}

// nbtValue type une valeur NBT d'après son type Go (décodeur nbt de gophertunnel).
func nbtValue(v reflect.Value) any {
	for v.Kind() == reflect.Interface || v.Kind() == reflect.Pointer {
		if v.IsNil() {
			return nil
		}
		v = v.Elem()
	}
	switch v.Kind() {
	case reflect.Int8, reflect.Uint8:
		return map[string]any{"byte": typedTree(v)}
	case reflect.Int16:
		return map[string]any{"short": v.Int()}
	case reflect.Int32:
		return map[string]any{"int": v.Int()}
	case reflect.Int64:
		return map[string]any{"long": v.Int()}
	case reflect.Float32:
		return map[string]any{"float": v.Float()}
	case reflect.Float64:
		return map[string]any{"double": v.Float()}
	case reflect.String:
		return map[string]any{"string": v.String()}
	case reflect.Map:
		return map[string]any{"compound": nbtCompound(v)}
	case reflect.Array:
		// TAG_Byte_Array, TAG_Int_Array, TAG_Long_Array : tableaux Go de taille fixe.
		values := make([]int64, v.Len())
		for i := range values {
			if e := v.Index(i); e.Kind() == reflect.Uint8 {
				values[i] = int64(int8(e.Uint()))
			} else {
				values[i] = e.Int()
			}
		}
		switch v.Type().Elem().Kind() {
		case reflect.Uint8, reflect.Int8:
			return map[string]any{"byte_array": values}
		case reflect.Int32:
			return map[string]any{"int_array": values}
		case reflect.Int64:
			return map[string]any{"long_array": values}
		}
		return map[string]any{"unknown": v.Type().String()}
	case reflect.Slice:
		// TAG_List : tranche, quel que soit le type des éléments.
		out := make([]any, v.Len())
		for i := range out {
			out[i] = nbtValue(v.Index(i))
		}
		return map[string]any{"list": out}
	default:
		return map[string]any{"unknown": v.Type().String()}
	}
}
