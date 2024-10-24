# fizz buzz - e.g. `100 enum *fizzbuzz queue print`
macro fizzbuzz
    fizzbuzz swap dup 15 mod 0 eq 3 rotr select
    buzz swap dup 5 mod 0 eq 3 rotr select
    fizz swap dup 3 mod 0 eq 3 rotr select
;

macro entity
    entities        # ent_name [entities..]
    dup 3 rotr      # [entities..] ent_name [entities..]
    name eq         # [entities..] name_filter
    filter
    0 index
;

macro fib
    macro fib_inner_leq1
        1 eq        # cond
        1 swap      # then
        0 swap      # else
        select
    ;
    macro fib_inner_geq2
        dup
        -1 add
        *fib_inner queue
        swap
        -2 add
        *fib_inner queue
        add
    ;
    macro fib_inner
        dup
        1 leq                   # cond
        fib_inner_leq1 swap     # then
        fib_inner_geq2 swap     # else
        select
        load queue
    ;

    *fib_inner queue
;

# alternate fib
macro fib
    macro fib_inner_leq1
        1 eq    # cond
        1       # then
        0       # else
        select
    ;
    macro fib_inner_geq2
        dup
        -1 add
        *fib_inner queue
        swap
        -2 add
        *fib_inner queue
        add
    ;
    macro fib_inner
        dup
        1 leq               # cond
        fib_inner_leq1      # then
        fib_inner_geq2      # else
        select
        load queue
    ;

    *fib_inner queue
;

macro fuzzent
    entities dup
    3 rotr
    name
    swap fuzzy
    filter
;

macro stacklen
    iter        # [stack..]
    dup len     # [stack..] len
    append      # [stack.. len]
    push        # stack.. len
;

macro for_each
    macro _for_each_finish      # macro_name
        pop
    ;
    macro _for_each_run_loop    # macro_name
        dup                     # macro_name
        *_for_each_iter_i       # macro_name macro_name i
        dup 1 add               # macro_name macro_name i (i+1)
        _for_each_iter_i store  # macro_name macro_name i
        *_for_each_iter         # macro_name macro_name i iter
        swap index              # macro_name macro_name iter[i]
        3 rotl load             # macro_name iter[i] [*macro_name..]
        *_for_each_inner        # macro_name iter[i] [*macro_name..] [*_for_each_inner..]
        extend                  # macro_name iter[i] [*macro_name.. *_for_each_inner..]
        queue
    ;
    macro _for_each_inner       # macro_name
        *_for_each_iter_i       # macro_name i
        *_for_each_iter_len     # macro_name i len
        geq                     # macro_name cond
        _for_each_finish        # macro_name cond _for_each_finish
        _for_each_run_loop      # macro_name cond _for_each_finish _for_each_run_loop
        select                  # macro_name _inner
        load queue
    ;
                                # iter macro_name
    swap                        # macro_name iter
    dup len 0                   # macro_name iter len 0
    _for_each_iter_i   store    # macro_name iter len
    _for_each_iter_len store    # macro_name iter
    _for_each_iter     store    # macro_name
    *_for_each_inner   queue
;

macro for_each_enum
    macro _for_each_finish      # macro_name
        pop
    ;
    macro _for_each_run_loop    # macro_name
        dup                     # macro_name
        *_for_each_iter_i       # macro_name macro_name i
        dup 1 add               # macro_name macro_name i (i+1)
        _for_each_iter_i store  # macro_name macro_name i
        dup *_for_each_iter     # macro_name macro_name i i iter
        swap index              # macro_name macro_name i iter[i]
        3 rotl load             # macro_name i iter[i] [*macro_name..]
        *_for_each_inner        # macro_name i iter[i] [*macro_name..] [*_for_each_inner..]
        extend                  # macro_name i iter[i] [*macro_name.. *_for_each_inner..]
        queue
    ;
    macro _for_each_inner       # macro_name
        *_for_each_iter_i       # macro_name i
        *_for_each_iter_len     # macro_name i len
        geq                     # macro_name cond
        _for_each_finish        # macro_name cond _for_each_finish
        _for_each_run_loop      # macro_name cond _for_each_finish _for_each_run_loop
        select                  # macro_name _inner
        load queue
    ;
                                # iter macro_name
    swap                        # macro_name iter
    dup len 0                   # macro_name iter len 0
    _for_each_iter_i   store    # macro_name iter len
    _for_each_iter_len store    # macro_name iter
    _for_each_iter     store    # macro_name
    *_for_each_inner   queue
;


macro empty_iter
    macro _temp ;
    *_temp
;

macro collect               # [iter..] a b c.. z
    macro _collect_inner    # [iter..] a b c.. z [vals..]
        swap                # [iter..] a b c.. [vals..] z
        dup type            # [iter..] a b c.. [vals..] z z_type
        "iter" eq           # [iter..] a b c.. [vals..] z z_is_iter
        _collect_finish
        _collect_collect
        select load queue
    ;
    macro _collect_collect  # [iter..] a b c.. [vals..] z
        append              # [iter..] a b c.. [vals.. z]
        *_collect_inner
        queue
    ;
    macro _collect_finish   # [vals..] [iter..]
        swap reverse        # [iter..] [slav..]
        extend              # [iter.. slav..]
    ;
    #                       # [iter..] a b c.. z
    *empty_iter queue       # [iter..] a b c.. z []
    *_collect_inner queue
;

macro map_type          # [iter..]
    macro "_map_type_type"  # i val
        type                # i val_type
        *_map_type_iter     # i val_types [types..]
        swap append         # i [types..]
        "_map_type_iter"
        store
        pop
    ;
    *empty_iter queue   # [iter..] []
    "_map_type_iter"    # [iter..] [] "_map_type_iter"
    store               # [iter..]
    "_map_type_type"
    *for_each queue
    *_map_type_iter
;

macro named_entities    #
    entities dup        # [entities..] [entities..]
    name *map_type queue
    "literal" eq
    filter
;

macro move_entity       # entity delta
    swap                # delta entity
    dup
    "bevy_transform::components::transform::Transform"
    get                 # delta entity transform
    dup
    "translation" get   # delta entity transform translation
    4 rotl              # entity transform translation delta
    add                 # entity transform translation
    "translation" swap  # entity transform "translation" translation
    set                 # entity transform
    add
;

macro print_all
    macro _print_all_print print pop ;
    iter                # [stack..]
    _print_all_print
    *for_each queue
;

macro drop
    iter pop
;

macro count_occurances  # [haystack..] needle
    eq dup filter len
;

macro "if"  # cond then else
    "_if_else" store
    "_if_then" store
    "_if_then"
    "_if_else"
    select
    load
;

macro rotation_id
    0 0 0 1 quat
;

macro "rotation_x"      # deg
    0.5 mul             # deg
    0.0174532925 mul    # rad
    dup                 # rad rad
    cos swap sin        # c s
    0 0                 # c s 0 0
    4 rotl              # s 0 0 c
    quat
;

macro "rotation_y"      # deg
    0.5 mul             # deg
    0.0174532925 mul    # rad
    0 swap              # 0 rad
    dup                 # 0 rad rad
    sin swap cos        # 0 s c
    0 swap              # 0 s 0 c
    quat
;

macro "rotation_z"      # deg
    0.5 mul             # deg
    0.0174532925 mul    # rad
    0 0 3 rotl          # 0 0 rad
    dup                 # 0 0 rad rad
    sin swap cos        # 0 0 s c
    quat
;

macro "rotate_x"        # entity deg
    swap                # deg entity
    dup
    "bevy_transform::components::transform::Transform"
    get                 # deg entity entity
    dup                 # deg entity transform transform
    "rotation" get      # deg entity transform rotation
    4 rotl              # entity transform rotation deg
    $"rotation_x"       # entity transform rotation rot_deg
    mul                 # entity transform rotation'
    "rotation" swap set # entity transform'
    add
;

macro "rotate_y"        # entity deg
    swap                # deg entity
    dup
    "bevy_transform::components::transform::Transform"
    get                 # deg entity entity
    dup                 # deg entity transform transform
    "rotation" get      # deg entity transform rotation
    4 rotl              # entity transform rotation deg
    $"rotation_y"       # entity transform rotation rot_deg
    mul                 # entity transform rotation'
    "rotation" swap set # entity transform'
    add
;

macro "rotate_z"        # entity deg
    swap                # deg entity
    dup
    "bevy_transform::components::transform::Transform"
    get                 # deg entity entity
    dup                 # deg entity transform transform
    "rotation" get      # deg entity transform rotation
    4 rotl              # entity transform rotation deg
    $"rotation_z"       # entity transform rotation rot_deg
    mul                 # entity transform rotation'
    "rotation" swap set # entity transform'
    add
;
