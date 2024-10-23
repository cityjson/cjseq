// use cjseq::CityJSON;
use cjseq::CityJSONFeature;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3::types::PyList;
extern crate cjseq;
use serde_json::Value;
use std::fmt::Write;

#[pymodule]
fn cjseqpy(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<CityJSON>()?;
    Ok(())
}

#[pyclass(unsendable)]
pub struct CityJSON {
    cjj: cjseq::CityJSON,
}

#[pymethods]
impl CityJSON {
    #[new]
    fn new(cj_string: String) -> PyResult<Self> {
        let j: cjseq::CityJSON = cjseq::CityJSON::from_str(&cj_string)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(CityJSON { cjj: j })
    }

    fn add_cjfeature_str(&mut self, cjf_string: String) -> PyResult<bool> {
        let mut j: CityJSONFeature = cjseq::CityJSONFeature::from_str(&cjf_string)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.cjj.add_cjfeature(&mut j);
        Ok(true)
    }

    fn add_cjfeature_json(&mut self, cjf: &PyDict) -> PyResult<bool> {
        let v: Value = convert_py_any_to_json(cjf)?;
        let mut j: CityJSONFeature = CityJSONFeature::from_value(v).unwrap();
        self.cjj.add_cjfeature(&mut j);
        Ok(true)
    }

    fn get_cj_str(&self) -> PyResult<String> {
        let mut s = String::new();
        write!(&mut s, "{}", serde_json::to_string(&self.cjj).unwrap());
        Ok(s)
    }

    fn get_cj_json(&self) -> PyResult<PyObject> {
        let v = serde_json::to_value(self.cjj.clone()).unwrap();
        Python::with_gil(|py| {
            let json_object = convert_json_value_to_pyobject(py, &v)?;
            Ok(json_object)
        })
    }

    fn get_metadata_json(&self) -> PyResult<PyObject> {
        let v = serde_json::to_value(self.cjj.get_metadata()).unwrap();
        Python::with_gil(|py| {
            let json_object = convert_json_value_to_pyobject(py, &v)?;
            Ok(json_object)
        })
    }

    fn get_cjfeature_str(&self, i: usize) -> PyResult<String> {
        let re = self.cjj.get_cjfeature(i);
        match re {
            Some(f) => {
                let mut s = String::new();
                write!(&mut s, "{}", serde_json::to_string(&f).unwrap());
                Ok(s)
            }
            None => Err(PyTypeError::new_err("no feature")),
        }
    }

    fn get_cjfeature_json(&self, i: usize) -> PyResult<PyObject> {
        let re = self.cjj.get_cjfeature(i);
        match re {
            Some(f) => {
                let v = serde_json::to_value(f).unwrap();
                // println!("{:?}", v);
                Python::with_gil(|py| {
                    let json_object = convert_json_value_to_pyobject(py, &v)?;
                    Ok(json_object)
                })
            }
            None => Err(PyTypeError::new_err("no feature")),
        }
    }

    //-- TODO: add sort_features
    //-- TODO: add remove_duplicate_vertices
    //-- TODO: add update_transform
}

fn convert_json_value_to_pyobject(py: Python, value: &Value) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.to_object(py)),
        Value::Number(num) => {
            if let Some(i) = num.as_i64() {
                Ok(i.to_object(py))
            } else if let Some(u) = num.as_u64() {
                Ok(u.to_object(py))
            } else if let Some(f) = num.as_f64() {
                Ok(f.to_object(py))
            } else {
                Err(pyo3::exceptions::PyTypeError::new_err("Invalid number"))
            }
        }
        Value::String(s) => Ok(s.to_object(py)),
        Value::Array(arr) => {
            let py_list = PyList::new(
                py,
                arr.iter()
                    .map(|v| convert_json_value_to_pyobject(py, v))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            Ok(py_list.to_object(py))
        }
        Value::Object(map) => {
            let py_dict = PyDict::new(py);
            for (k, v) in map {
                py_dict.set_item(k, convert_json_value_to_pyobject(py, v)?)?;
            }
            Ok(py_dict.to_object(py))
        }
    }
}

fn convert_py_any_to_json(py_any: &PyAny) -> PyResult<Value> {
    if py_any.is_none() {
        Ok(Value::Null)
    } else if let Ok(b) = py_any.extract::<bool>() {
        Ok(Value::Bool(b))
    } else if let Ok(i) = py_any.extract::<i64>() {
        Ok(Value::Number(i.into()))
    } else if let Ok(f) = py_any.extract::<f64>() {
        Ok(Value::Number(serde_json::Number::from_f64(f).ok_or_else(
            || PyTypeError::new_err("Invalid float value"),
        )?))
    } else if let Ok(s) = py_any.extract::<String>() {
        Ok(Value::String(s))
    } else if let Ok(py_list) = py_any.downcast::<PyList>() {
        let mut vec = Vec::new();
        for item in py_list {
            vec.push(convert_py_any_to_json(item)?);
        }
        Ok(Value::Array(vec))
    } else if let Ok(py_dict) = py_any.downcast::<PyDict>() {
        let mut map = serde_json::Map::new();
        for (key, value) in py_dict {
            let key = key.extract::<String>()?;
            let value = convert_py_any_to_json(value)?;
            map.insert(key, value);
        }
        Ok(Value::Object(map))
    } else {
        Err(PyTypeError::new_err("Unsupported type"))
    }
}
