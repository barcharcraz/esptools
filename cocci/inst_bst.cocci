// typedefs are there to make devel easier
// so one can develop with concrete types,
// no need to keep them.
@rm_kty@
type keyty;
@@
-typedef keyty keytype;

@rm_valty@
type valty;
@@
-typedef valty valtype;

@@
type rm_kty.keyty;
typedef keytype;
@@
-keytype
+keyty

@@
type rm_valty.valty;
typedef valtype;
@@
-valtype
+valty

@s@
identifier func;
fresh identifier new_func = "esp_int_" ## func;
@@
-func
+new_func
 (...) {...}

@@
identifier s.func;
identifier s.new_func;
@@
-func
+new_func