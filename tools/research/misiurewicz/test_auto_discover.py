"""FIX-47: native binary64 coefficients, exact camera transport and fail-closed build."""
import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import mpmath as mp
import auto_discover as auto


class NativeModelTests(unittest.TestCase):
    def test_real_mpmath_phase_preserves_deep_camera_and_scaled_bias(self):
        re="-0.743643887037158704752191506114774"
        im="0.131825904205311970493132056385139"
        with mp.workdps(1100), tempfile.TemporaryDirectory() as tmp:
            k=dict(radius=0.125,dp=0j,dlam=0j,rho=2+0j,dk=[0j],q=1,p=1,
                   bias_hp=mp.mpf("1e-1000"),phases_hp=[mp.mpf("0.5"),mp.mpc("0.25","-0.125")],
                   zref=[0j,1j],k=[0j,1+0j])
            file=Path(tmp)/"model.txt"
            auto.write_native_model(k,file,"1e-1000",32,re,im)
            lines=file.read_text().splitlines()
            self.assertIn("center_re "+re,lines)
            self.assertIn("center_im "+im,lines)
            self.assertIn("width 1e-1000",lines)
            self.assertIn("phase 0 0.5 0",lines)
            self.assertIn("phase 1 0.25 -0.125",lines)
            bias=next(l for l in lines if l.startswith("bias ")).split()
            reconstructed=mp.mpf(bias[1])*mp.power(2,int(bias[3]))
            self.assertLess(abs(reconstructed/mp.mpf("1e-1000")-1),mp.mpf("1e-15"))

    def test_discovery_and_build_errors_report_decline_before_native(self):
        for stage in ("discovery","build"):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as tmp:
                argv=["auto_discover.py","--re","0","--im","1","--width","1e-95",
                      "--fd","must-not-run","--native","must-not-run","--out",tmp]
                discovery=RuntimeError("discovery fault") if stage=="discovery" else None
                with patch("sys.argv",argv), patch.object(auto,"discover",side_effect=discovery,
                          return_value=({},dict(discovery_seconds=0.0,build_seconds=0.0))), \
                     patch.object(auto,"write_native_model",side_effect=TypeError("build fault")), \
                     patch.object(auto.subprocess,"run") as native, contextlib.redirect_stdout(io.StringIO()) as stdout:
                    auto.main()
                report=json.loads((Path(tmp)/"report.json").read_text())
                self.assertEqual(report["pre_render_decision"],"decline")
                self.assertEqual(report["decision"],"decline")
                self.assertEqual(report["error_stage"],stage)
                self.assertIn(stage+" fault",report["reason"])
                self.assertEqual(json.loads(stdout.getvalue()),report)
                native.assert_not_called()


if __name__=="__main__":
    unittest.main()
