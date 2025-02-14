#include <stdio.h>
#include<stdlib.h>

int main(int argc, char* argv[]) {


	// Library "builtin" type definitions --------------------------------------------------------------------------------

	
	typedef struct {
		void* u_internal_name;
	} group_23;
	
	typedef struct {
		char empty;
	} group_1;
	
	typedef struct {
		char empty;
	} group_26;
	
	typedef struct {
		void* u_compared_to;
	} group_52;
	
	typedef struct {
		char empty;
	} group_4;
	
	
	typedef struct {
	} object_57;
	
	typedef struct {
		char empty;
	} group_18;
	
	typedef struct {
		char empty;
	} group_10;
	
	typedef struct {
		void* u_parameters;
		void* u_return_type;
		void* u_compile_time_parameters;
		void* u_tags;
		void* u_this_object;
	} group_14;
	
	typedef struct {
	} object_58;
	
	
	
	
	typedef struct {
		char empty;
	} group_13;
	
	
	typedef struct {
		void* u_equals;
	} group_44;
	
	typedef struct {
		void* u_reason;
	} group_15;
	
	typedef struct {
	} object_59;
	
	typedef struct {
		void* u_plus;
	} group_32;
	
	typedef struct {
		char empty;
	} group_6;
	
	
	
	typedef struct {
		void* u_name;
		void* u_type;
	} group_12;
	
	typedef struct {
		void* u_fields;
	} group_11;
	
	
	typedef struct {
		void* u_variants;
	} group_19;
	
	
	
	typedef struct {
		void* u_input;
		void* u_print;
	} object_64;
	
	typedef struct {
		void* u_terminal;
	} object_65;
	
	typedef struct {
		void* u_to_text;
	} group_9;
	
	typedef struct {
		void* u_runtime;
		void* u_BuiltinTag;
		void* u_This;
		void* u_Number;
		void* u_Compareable;
		void* u_RepresentAs;
		void* u_Ordering;
		void* u_system_side_effects;
		void* u_List;
		void* u_Object;
		void* u_Function;
		void* u_no_side_effects;
		void* u_Attempted;
		void* u_Nothing;
		void* u_Optional;
		void* u_OneOf;
		void* u_builtin_function;
		void* u_Equalable;
		void* u_RuntimeTag;
		void* u_default;
		void* u_Addable;
		void* u_Text;
		void* u_AddNumbers;
		void* u_Boolean;
		void* u_Parameter;
		void* u_Group;
		void* u_CompareablesAreEqualable;
		void* u_Either;
		void* u_system;
		void* u_Anything;
	} object_66;

	// Library "builtin" value definitions --------------------------------------------------------------------------------

	group_14 literal_17 = (group_14) {
	};
	
	group_11 literal_23 = (group_11) {
	};
	
	group_11 literal_1 = (group_11) {
	};
	
	group_11 literal_26 = (group_11) {
	};
	
	group_11 literal_52 = (group_11) {
	};
	
	group_11 literal_4 = (group_11) {
	};
	
	group_19 literal_48 = (group_19) {
	};
	
	object_57 literal_57 = (object_57) {
	};
	
	group_11 literal_18 = (group_11) {
	};
	
	group_11 literal_10 = (group_11) {
	};
	
	group_11 literal_14 = (group_11) {
	};
	
	object_58 literal_58 = (object_58) {
	};
	
	group_13 literal_37 = (group_13) {
	};
	
	group_19 literal_3 = (group_19) {
	};
	
	group_13 literal_5 = (group_13) {
	};
	
	group_11 literal_13 = (group_11) {
	};
	
	group_14 literal_25 = (group_14) {
	};
	
	group_11 literal_44 = (group_11) {
	};
	
	group_11 literal_15 = (group_11) {
	};
	
	object_59 literal_59 = (object_59) {
	};
	
	group_11 literal_32 = (group_11) {
	};
	
	group_11 literal_6 = (group_11) {
	};
	
	group_4 literal_36 = (group_4) {
	};
	
	group_19 literal_22 = (group_19) {
	};
	
	group_11 literal_12 = (group_11) {
	};
	
	group_11 literal_11 = (group_11) {
	};
	
	group_4 literal_56 = (group_4) {
	};
	
	group_11 literal_19 = (group_11) {
	};
	
	group_14 literal_40 = (group_14) {
	};
	
	group_14 literal_0 = (group_14) {
	};
	
	object_64 literal_64 = (object_64) {
		.u_input = &literal_40,
		.u_print = &literal_39,
	};
	
	object_65 literal_65 = (object_65) {
		.u_terminal = &literal_64,
	};
	
	group_11 literal_9 = (group_11) {
	};
	
	object_66 literal_66 = (object_66) {
		.u_runtime = &literal_17,
		.u_BuiltinTag = &literal_23,
		.u_This = &literal_1,
		.u_Number = &literal_26,
		.u_Compareable = &literal_52,
		.u_RepresentAs = &literal_4,
		.u_Ordering = &literal_48,
		.u_system_side_effects = &literal_57,
		.u_List = &literal_18,
		.u_Object = &literal_10,
		.u_Function = &literal_14,
		.u_no_side_effects = &literal_58,
		.u_Attempted = &literal_37,
		.u_Nothing = &literal_3,
		.u_Optional = &literal_5,
		.u_OneOf = &literal_13,
		.u_builtin_function = &literal_25,
		.u_Equalable = &literal_44,
		.u_RuntimeTag = &literal_15,
		.u_default = &literal_59,
		.u_Addable = &literal_32,
		.u_Text = &literal_6,
		.u_AddNumbers = &literal_36,
		.u_Boolean = &literal_22,
		.u_Parameter = &literal_12,
		.u_Group = &literal_11,
		.u_CompareablesAreEqualable = &literal_56,
		.u_Either = &literal_19,
		.u_system = &literal_65,
		.u_Anything = &literal_9,
	};
	
	
	
	
	
	
	
	
	
	
	void* u_Text;
	u_Text = &literal_6;
	label_end_u_Text:;
	
	
	void* u_system;
	u_system = &literal_65;
	label_end_u_system:;
	
	
	void* u_terminal;
	u_terminal = &literal_64;
	label_end_u_terminal:;
	
	
	void* u_print;
	u_print = &literal_39;
	label_end_u_print:;
	


	return 0;
}